use std::collections::HashMap;

use async_trait::async_trait;
use uuid::Uuid;

use crate::user::{User, UserId};

pub type PartyId = Uuid;

#[async_trait]
pub trait PartyPersistence {
    async fn create_party(&mut self, party: Party) -> anyhow::Result<()>;
    async fn get_user_role(&self, user_id: UserId, party_id: PartyId) -> anyhow::Result<Role>;
    async fn is_user_in_party(&self, user_id: UserId, party_id: PartyId) -> anyhow::Result<bool>;
    async fn get_party_by_id(&self, id: PartyId) -> Option<&Party>;
    async fn get_party_by_id_mut(&mut self, id: PartyId) -> Option<&mut Party>;
    async fn add_user_to_party(
        &mut self,
        user_id: UserId,
        party_id: PartyId,
        member_role: Role,
    ) -> anyhow::Result<()>;
    async fn remove_user_from_party(
        &mut self,
        user_id: UserId,
        party_id: PartyId,
    ) -> anyhow::Result<()>;
    async fn delete_party(&mut self, party_id: PartyId) -> anyhow::Result<Party>;
}

#[derive(Clone, Debug)]
pub struct Party {
    pub id: PartyId,
    pub creator: User,
    /// Users that have joined this party
    pub members: HashMap<UserId, Role>,
}

impl PartialEq for Party {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Party {
    pub fn create(sender: &User) -> Self {
        // The only starting member of a party is it's creator
        let mut members = HashMap::new();
        members.insert(sender.id, Role::Creator);

        Self {
            id: Uuid::new_v4(),
            creator: sender.clone(),
            members,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Role {
    Creator,
    Admin,
    Default,
}
