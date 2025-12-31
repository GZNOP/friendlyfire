use std::sync::Arc;

use anyhow::Result;
use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use axum::{
    Json, Router,
    extract::{
        Request, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
};
use friendlyfire_shared_lib::{ClientMessage, ClientMessageType, SenderInfo, ServerMessage};
use log::{info, trace};
use serde::{Deserialize, Serialize};
use tokio::{net::TcpListener, sync::RwLock};
use uuid::Uuid;

use crate::{
    jwt,
    party::{Party, Role},
    store::{MockStore, Store},
    user::User,
};

struct AppState {
    store: RwLock<MockStore>,
}

pub struct HTTPServer {
    state: Arc<AppState>,
}

impl HTTPServer {
    pub fn new(store: MockStore) -> Self {
        Self {
            state: Arc::new(AppState {
                store: RwLock::new(store),
            }),
        }
    }

    pub async fn serve(self, addr: &str) -> Result<()> {
        let router = Router::new()
            .route("/ws", get(ws))
            .layer(middleware::from_fn(jwt_auth))
            .route("/login", post(login))
            .route("/register", post(register))
            .with_state(self.state);

        let listener = TcpListener::bind(addr).await?;
        axum::serve(listener, router).await?;

        Ok(())
    }
}

#[derive(Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct TokenResponse {
    jwt: String,
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<TokenResponse>, StatusCode> {
    let store = state.store.read().await;
    let user = store
        .get_user_by_email(&payload.email)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let parsed_hash =
        PasswordHash::new(&user.password_hash).map_err(|_| StatusCode::UNAUTHORIZED)?;

    Argon2::default()
        .verify_password(payload.password.as_bytes(), &parsed_hash)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let token = jwt::issue_jwt(user.id).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    info!("Successful login for user: {}", user);

    Ok(Json(TokenResponse { jwt: token }))
}

#[derive(Deserialize)]
struct RegisterRequest {
    email: String,
    username: String,
    password: String,
}

async fn register(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterRequest>,
) -> StatusCode {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = match argon2
        .hash_password(payload.password.as_bytes(), &salt)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    {
        Ok(hash) => hash.to_string(),
        Err(error) => return error,
    };

    let user = User::new(payload.email, payload.username, password_hash);
    let mut store = state.store.write().await;

    if store.create_user(user.clone()).is_err() {
        return StatusCode::UNAUTHORIZED;
    }

    info!("Successfully registered user: {}", user);

    StatusCode::CREATED
}

async fn jwt_auth(req: Request, next: Next) -> Result<Response, StatusCode> {
    let headers = req.headers();

    if let Some(auth_header) = headers.get("Authorization")
        && let Ok(auth_str) = auth_header.to_str()
        && let Some(token) = auth_str.strip_prefix("Bearer ")
    {
        if jwt::validate_jwt(token).is_ok() {
            return Ok(next.run(req).await);
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}

async fn ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    request: Request,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state, request))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>, request: Request) {
    let jwt = jwt::decode_header(request.headers().get("Authorization").unwrap()).unwrap();
    let user_id = jwt.claims.user_id();

    let sender = {
        let store = state.store.read().await;
        store.get_user_by_id(user_id).unwrap().clone()
    };

    let server_info = SenderInfo { id: Uuid::new_v4() };

    while let Some(Ok(msg)) = socket.recv().await {
        trace!("Message received {:?}", msg);

        match msg {
            Message::Text(msg) => {
                let message: ClientMessage = serde_json::from_str(&msg).unwrap();

                match message.kind {
                    ClientMessageType::CreateParty => {
                        let mut store = state.store.write().await;
                        store.create_party(Party::create(&sender)).unwrap();

                        let reply = ServerMessage::party_created(server_info.clone());
                        socket
                            .send(Message::Text(serde_json::to_string(&reply).unwrap().into()))
                            .await
                            .unwrap();
                    }

                    ClientMessageType::JoinParty { invitation_token } => {
                        let mut store = state.store.write().await;
                        let invitation = store.consume_invitation(invitation_token);

                        match invitation {
                            Ok(party_id) => {
                                store
                                    .add_user_to_party(sender.id, party_id, Role::Default)
                                    .unwrap();
                            }
                            Err(_) => {
                                let err = ServerMessage::error_message("Invalid or expired token");
                                socket
                                    .send(Message::Text(
                                        serde_json::to_string(&err).unwrap().into(),
                                    ))
                                    .await
                                    .unwrap();
                            }
                        }
                    }

                    _ => {}
                }
            }

            Message::Close(_) => break,
            _ => {}
        }
    }
}
