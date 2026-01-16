use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    DisplayOptions, Overlay, OverlayDescriptor,
    message::{builder::MessageBuilder, version::Version},
};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
/// Top-level message emitted by the server
/// Server messages are authoritative and should never be rejected or altered by clients
pub struct ServerMessage {
    /// Friendlyfire protocol version of the server used to send the message.
    /// Used to detect incompatibilities between client and server.
    pub version: Version,

    /// Information about the sender of the original message that induced this one.
    /// Some when it is a reply to a `ClientMessage`.
    /// None when the server sends a standalone message.
    pub sender: Option<SenderInfo>,

    // Flattened to avoid a "kind" object in the message that isn't really useful.
    /// Actual message payload.
    #[serde(flatten)]
    pub kind: ServerMessageType,
}

// TODO : Could be expanded a subset of `User` attributes.
// `let senderInfo = User.into()` should be possible
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SenderInfo {
    /// Stable server-assigned identifier of the sender.
    pub id: Uuid,
}

impl SenderInfo {
    pub fn new(id: Uuid) -> Self {
        Self { id }
    }
}

/// All possible server message kinds.
///
/// When a `ClientMessageType` is received on the server it is always converted into a `ServerMessage` of the same enum value.
/// e.g. : `ClientMessageType::Fire` becomes when he is relayed through the server `ServerMessageType::Fire`
/// This garanties the message to be authoritative and right (no foul play by the client)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum ServerMessageType {
    AuthOk,

    RegisterOk {
        /// Newly create JWT token
        jwt: String,
    },

    /// Response to `ClientMessageType::CreateParty`
    /// Confirms party creation and assigns creator(same as admin) privileges to the requesting client.
    PartyCreated,

    /// Response to `ClientMessageType::DisbandParty`
    /// Confirms party deletion.
    /// NOTE : This can only be done by the party creator (see `Role::Creator`)
    PartyDisbanded,

    /// Response to `ClientMessageType::JoinParty`
    /// This tells the client he successfully joined the given `Party`
    JoinAccepted,

    /// Response to `ClientMessageType::CreateInvitationToken`
    /// This tells the client he successfully joined the given `Party`
    InvitationTokenCreated {
        token: Uuid,
    },

    OverlaysAvailable {
        overlays: Vec<OverlayDescriptor>,
        options: DisplayOptions,
    },

    /// Aggregate of `ClientMessageType::OverlaysAck`
    /// Sent when all online members of a party have downloaded the `Overlays`
    OverlaysFullAck,

    /// Aggregate of `ClientMessageType::RasterizationAck`
    /// Sent when all online members of a party have rasterized all the `Overlays`
    RasterizationFullAck,

    /// Relay of the `ClientMessageType::Fire`
    /// Can only be sent once `OverlaysFullAck` and `RasterizationFullAck` have been emitted
    Fire {
        party_id: Uuid,
        job_id: Uuid,
    },

    /// Error emitted by the server.
    /// Indicates a rejected client action or a server error.
    Error {
        message: String,
    },
}

impl ServerMessage {
    fn system() -> ServerMessageBuilder {
        ServerMessageBuilder {
            version: Version::current(),
            sender: None,
            kind: None,
        }
    }

    // sender should be Some
    fn relay(sender: SenderInfo) -> ServerMessageBuilder {
        ServerMessageBuilder {
            version: Version::current(),
            sender: Some(sender),
            kind: None,
        }
    }

    //
    // Shortcuts utils
    //

    pub fn auth_ok(sender: SenderInfo) -> Self {
        Self::relay(sender).kind(ServerMessageType::AuthOk).build()
    }

    pub fn register_ok(sender: SenderInfo, jwt: String) -> Self {
        Self::relay(sender)
            .kind(ServerMessageType::RegisterOk { jwt })
            .build()
    }

    pub fn party_created(sender: SenderInfo) -> Self {
        Self::relay(sender)
            .kind(ServerMessageType::PartyCreated)
            .build()
    }

    pub fn party_disbanded(sender: SenderInfo) -> Self {
        Self::relay(sender)
            .kind(ServerMessageType::PartyDisbanded)
            .build()
    }

    pub fn join_accepted(sender: SenderInfo) -> Self {
        Self::relay(sender)
            .kind(ServerMessageType::JoinAccepted)
            .build()
    }

    pub fn invitation_token(sender: SenderInfo, token: Uuid) -> Self {
        Self::relay(sender)
            .kind(ServerMessageType::InvitationTokenCreated { token })
            .build()
    }

    pub fn overlays_available(
        sender: SenderInfo,
        overlays: Vec<OverlayDescriptor>,
        options: DisplayOptions,
    ) -> Self {
        Self::relay(sender)
            .kind(ServerMessageType::OverlaysAvailable { overlays, options })
            .build()
    }

    pub fn overlays_full_ack(sender: SenderInfo) -> Self {
        Self::relay(sender)
            .kind(ServerMessageType::OverlaysFullAck)
            .build()
    }

    pub fn rasterization_full_ack(sender: SenderInfo) -> Self {
        Self::relay(sender)
            .kind(ServerMessageType::RasterizationFullAck)
            .build()
    }

    pub fn fire(sender: SenderInfo, party_id: Uuid, job_id: Uuid) -> Self {
        Self::relay(sender)
            .kind(ServerMessageType::Fire { party_id, job_id })
            .build()
    }

    // Weird name, but at least there is no conflict
    pub fn error_message(message: impl Into<String>) -> Self {
        Self::system()
            .kind(ServerMessageType::Error {
                message: message.into(),
            })
            .build()
    }
}

struct ServerMessageBuilder {
    version: Version,
    sender: Option<SenderInfo>,
    kind: Option<ServerMessageType>,
}

impl MessageBuilder for ServerMessageBuilder {
    type Message = ServerMessage;
    type MessageKind = ServerMessageType;

    fn kind(mut self, kind: ServerMessageType) -> Self {
        self.kind = Some(kind);
        self
    }

    fn build(self) -> ServerMessage {
        ServerMessage {
            version: self.version,
            sender: self.sender,
            kind: self.kind.expect("Message kind must be set"),
        }
    }
}
