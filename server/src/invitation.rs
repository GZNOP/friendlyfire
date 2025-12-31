use std::time::Duration;

use tokio::time::Instant;
use uuid::Uuid;

use crate::party::PartyId;

pub type InvitationToken = Uuid;

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
