use std::time::Duration;

use async_trait::async_trait;
use tokio::time::Instant;
use uuid::Uuid;

use crate::party::PartyId;

pub type InvitationToken = Uuid;

#[async_trait]
pub trait InvitationPersistence {
    async fn create_invitation(
        &mut self,
        party_id: PartyId,
        ttl: Duration,
    ) -> anyhow::Result<InvitationToken>;
    async fn consume_invitation(&mut self, token: InvitationToken) -> anyhow::Result<PartyId>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Invitation {
    pub token: InvitationToken,
    pub valid_until: Instant,
    pub party_id: PartyId,
}

impl Invitation {
    pub fn new(ttl: Duration, party_id: PartyId) -> Self {
        Self {
            token: Uuid::new_v4(),
            valid_until: Instant::now() + ttl,
            party_id,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.valid_until > Instant::now()
    }
}
