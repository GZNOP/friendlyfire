use std::{collections::HashSet, io, sync::Arc, time::Duration};
use tokio_util::io::{ReaderStream, StreamReader};

use anyhow::Result;
use axum::{
    Json, Router,
    body::Body,
    extract::{
        DefaultBodyLimit, Multipart, Path, Request, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, StatusCode},
    middleware::{self},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use friendlyfire_shared_lib::{
    ClientMessage, ClientMessageType, DisplayOptions, Overlay, OverlayDescriptor, SenderInfo,
    ServerMessage,
};
use futures_util::{SinkExt, StreamExt, TryStreamExt};
use log::warn;
use serde::Deserialize;
use tokio::{net::TcpListener, sync::mpsc::unbounded_channel};
use uuid::Uuid;

use crate::{
    http::{
        auth::{login, register},
        jwt,
        middleware::jwt_auth,
        state::AppState,
        storage::{OverlayId, OverlayStorage},
    },
    invitation::{InvitationPersistence, InvitationToken},
    party::{Party, PartyId, PartyPersistence, Role},
    user::UserPersistence,
};

pub struct HTTPServer {
    state: Arc<AppState>,
}

impl HTTPServer {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    pub async fn serve(self, addr: &str) -> Result<()> {
        let router = Router::new()
            .route("/party/{party_id}", get(party_ws))
            .route("/party/{party_id}/create", post(create_party))
            .route("/party/{party_id}/join", post(join_party))
            .route("/party/{party_id}/invite", post(create_invitation))
            .route("/party/{party_id}/overlays_upload", post(upload_overlays)) // 200 Mb max
            .route(
                "/party/{party_id}/overlays_download",
                post(download_overlays).layer(DefaultBodyLimit::max(209715200)),
            )
            .route("/party/{party_id}/disband", post(disband_party))
            .layer(middleware::from_fn(jwt_auth))
            .route("/login", post(login))
            .route("/register", post(register))
            .with_state(self.state);

        let listener = TcpListener::bind(addr).await?;
        axum::serve(listener, router).await?;

        Ok(())
    }
}

#[axum::debug_handler]
async fn party_ws(
    ws: WebSocketUpgrade,
    Path(party_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    request: Request,
) -> Response {
    ws.on_upgrade(move |socket| handle_party_socket(socket, state, party_id, request))
}

async fn handle_party_socket(
    socket: WebSocket,
    state: Arc<AppState>,
    party_id: PartyId,
    request: Request,
) {
    let jwt = jwt::decode_header(request.headers().get("Authorization").unwrap()).unwrap();
    let sender_id = jwt.claims.user_id();

    // Validate membership
    {
        let store = state.store.read().await;
        if !store.is_user_in_party(sender_id, party_id).await.unwrap() {
            // Unauthorized
            return;
        }
    }

    let (mut ws_tx, mut ws_rx) = socket.split();

    let (tx, mut rx) = unbounded_channel::<ServerMessage>();
    state.ws.add(party_id, sender_id, tx).await;

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_tx
                .send(Message::Text(serde_json::to_string(&msg).unwrap().into()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    while let Some(Ok(message)) = ws_rx.next().await {
        match message {
            Message::Text(utf8_bytes) => {
                let message =
                    serde_json::from_slice::<ClientMessage>(utf8_bytes.as_bytes()).unwrap();
                match message.kind {
                    ClientMessageType::CreateParty => unreachable!(),
                    ClientMessageType::DisbandParty { party_id: _ } => unreachable!(),
                    ClientMessageType::JoinParty {
                        invitation_token: _,
                    } => unreachable!(),
                    ClientMessageType::CreateInvitationToken { party_id: _ } => unreachable!(),
                    ClientMessageType::Overlays {
                        overlays: _,
                        options: _,
                    } => todo!(),
                    ClientMessageType::OverlaysAck { job_id } => {
                        // if the job is finished send confirmation to the original sender of the overlays
                        if let Some(job_completed) = state.jobs.ack(job_id, sender_id).await {
                            let message = ServerMessage::overlays_full_ack(SenderInfo::new(
                                job_completed.sender,
                            ));
                            state.ws.send_to(job_completed.party_id, message).await;
                        }
                    }
                    ClientMessageType::RasterizationAck { job_id } => {
                        // if the job is finished send confirmation to the original sender of the overlays
                        if let Some(job_completed) = state.jobs.ack(job_id, sender_id).await {
                            let message = ServerMessage::rasterization_full_ack(SenderInfo::new(
                                job_completed.sender,
                            ));
                            state.ws.send_to(job_completed.party_id, message).await;
                        }
                    }
                    ClientMessageType::Fire { party_id, job_id } => {
                        let message =
                            ServerMessage::fire(SenderInfo::new(sender_id), party_id, job_id);
                        state.ws.broadcast(party_id, message).await;
                    }
                    ClientMessageType::Error { message } => {
                        warn!("User {} had an error : {}", sender_id, message)
                    }
                }
            }
            Message::Binary(_bytes) => unreachable!(),
            // Ping messages will be automatically responded to by the server, so you do not have to worry
            // about dealing with them yourself.
            Message::Ping(_bytes) => {}
            // Pong messages will be automatically sent to the client if a ping message is received, so
            // you do not have to worry about constructing them yourself unless you want to implement a
            // [unidirectional heartbeat](https://tools.ietf.org/html/rfc6455#section-5.5.3).
            Message::Pong(_bytes) => {}
            Message::Close(_close_frame) => todo!(),
        }
    }
}

#[derive(Deserialize)]
pub struct CreatePartyPayload {
    pub name: String,
}

#[axum::debug_handler]
pub async fn create_party(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let jwt = jwt::decode_header(headers.get("Authorization").unwrap()).unwrap();
    let sender_id = jwt.claims.user_id();

    let mut store = state.store.write().await;

    // get the user from the store
    let creator = match store.get_user_by_id(sender_id).await {
        Some(u) => u,
        None => return (StatusCode::UNAUTHORIZED, "User not found").into_response(),
    };

    let party = Party::create(creator);

    if let Err(e) = store.create_party(party).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }

    (
        StatusCode::CREATED,
        serde_json::to_string(&ServerMessage::party_created(SenderInfo::new(sender_id))).unwrap(),
    )
        .into_response()
}

#[derive(Deserialize)]
pub struct JoinPartyPayload {
    pub invitation_token: InvitationToken,
}

#[axum::debug_handler]
async fn join_party(
    Path(party_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<JoinPartyPayload>,
) -> impl IntoResponse {
    let jwt = jwt::decode_header(headers.get("Authorization").unwrap()).unwrap();
    let sender_id = jwt.claims.user_id();

    let mut store = state.store.write().await;

    // consume the invitation token
    match store.consume_invitation(payload.invitation_token).await {
        Ok(token_party_id) => {
            if token_party_id != party_id {
                return (StatusCode::FORBIDDEN, "Token does not belong to this party")
                    .into_response();
            }
        }
        Err(e) => return (StatusCode::FORBIDDEN, e.to_string()).into_response(),
    }

    // add user to party
    if let Err(e) = store
        .add_user_to_party(sender_id, party_id, Role::Default)
        .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }

    (StatusCode::OK, "Joined party successfully").into_response()
}

#[derive(Deserialize)]
pub struct CreateInvitationPayload {
    pub ttl_seconds: u64, // token lifetime
}

#[axum::debug_handler]
pub async fn create_invitation(
    Path(party_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<CreateInvitationPayload>,
) -> impl IntoResponse {
    let jwt = jwt::decode_header(headers.get("Authorization").unwrap()).unwrap();
    let sender_id = jwt.claims.user_id();

    let mut store = state.store.write().await;

    // check if user is the creator
    let party = match store.get_party_by_id(party_id).await {
        Some(p) => p,
        None => return (StatusCode::NOT_FOUND, "Party not found").into_response(),
    };

    if party.creator.id != sender_id {
        return (
            StatusCode::FORBIDDEN,
            "Only the creator can create invitations",
        )
            .into_response();
    }

    // create invitation with duration hardcap of 7 days
    let ttl = match payload.ttl_seconds > 604800 {
        true => Duration::from_secs(604800),
        false => Duration::from_secs(payload.ttl_seconds),
    };
    let token = match store.create_invitation(party_id, ttl).await {
        Ok(t) => t,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    (
        StatusCode::CREATED,
        serde_json::to_string(&ServerMessage::invitation_token(
            SenderInfo::new(sender_id),
            token,
        ))
        .unwrap(),
    )
        .into_response()
}

#[derive(Deserialize)]
pub struct PostOverlaysPayload {
    overlays: Vec<Overlay>,
    options: DisplayOptions,
}

#[axum::debug_handler]
pub async fn upload_overlays(
    Path(party_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let jwt = jwt::decode_header(headers.get("Authorization").unwrap()).unwrap();
    let sender_id = jwt.claims.user_id();

    // validation
    {
        let store = state.store.read().await;
        if !store.is_user_in_party(sender_id, party_id).await.unwrap() {
            return StatusCode::FORBIDDEN.into_response();
        }
    }

    let mut options: Option<DisplayOptions> = None;
    let mut stored_overlays = Vec::new();

    // upload on the server at /data/parties/{party_id}/{overlay_id}.bin

    while let Some(field) = multipart.next_field().await.unwrap() {
        match field.name() {
            Some("options") => {
                let bytes = field.bytes().await.unwrap();
                options = Some(serde_json::from_slice(&bytes).unwrap());
            }

            Some("file") => {
                let filename = field.file_name().unwrap_or("overlay").to_string();

                let content_type = field.content_type().map(|s| s.to_string());

                let mut reader =
                    StreamReader::new(field.map_err(|e| io::Error::new(io::ErrorKind::Other, e)));

                let overlay = state
                    .overlays
                    .create_overlay(party_id, filename, content_type, &mut reader)
                    .await
                    .unwrap();

                stored_overlays.push(overlay);
            }

            _ => {}
        }
    }

    let options = options
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing display options"))
        .unwrap();

    let overlay_descriptors: Vec<OverlayDescriptor> = stored_overlays
        .iter()
        .map(|o| OverlayDescriptor {
            id: o.id,
            url: format!("/overlays/{}", o.id),
            filename: o.filename.clone(),
            size: o.size,
        })
        .collect();

    let users_in_party: Vec<Uuid> = state.ws.users_in_party(party_id).await;
    let participants: HashSet<Uuid> = users_in_party.into_iter().collect();
    // Keep track of who has downloaded the overlays
    state
        .jobs
        .new_overlay_job(party_id, sender_id, participants.clone())
        .await;
    // Keep track of who has finished rasterizing the overlays
    state
        .jobs
        .new_rasterization_job(party_id, sender_id, participants.clone())
        .await;

    // Broadcast that overlays are ready to download
    state
        .ws
        .broadcast(
            party_id,
            ServerMessage::overlays_available(
                SenderInfo::new(sender_id),
                overlay_descriptors,
                options,
            ),
        )
        .await;

    (StatusCode::ACCEPTED).into_response()
}

#[axum::debug_handler]
async fn download_overlays(
    Path(overlay_id): Path<OverlayId>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.overlays.open_overlay(overlay_id).await {
        Ok(reader) => Response::builder()
            .status(StatusCode::OK)
            .body(Body::from_stream(ReaderStream::new(reader)))
            .unwrap(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

#[axum::debug_handler]
pub async fn disband_party(
    Path(party_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let jwt = jwt::decode_header(headers.get("Authorization").unwrap()).unwrap();
    let sender_id = jwt.claims.user_id();

    let mut store = state.store.write().await;

    // get the party
    let party = match store.get_party_by_id(party_id).await {
        Some(p) => p,
        None => return (StatusCode::NOT_FOUND, "Party not found").into_response(),
    };

    // check if sender is creator
    if party.creator.id != sender_id {
        return (
            StatusCode::FORBIDDEN,
            "Only the creator can disband this party",
        )
            .into_response();
    }

    // delete the party
    match store.delete_party(party_id).await {
        Ok(_) => (StatusCode::OK, "Party disbanded successfully").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
