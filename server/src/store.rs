use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::{anyhow, bail};
use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use async_trait::async_trait;
use tokio::time::Instant;

use crate::{
    invitation::{Invitation, InvitationPersistence, InvitationToken},
    party::{Party, PartyId, PartyPersistence, Role},
    user::{User, UserId, UserPersistence},
};

#[async_trait::async_trait]
pub trait DebugStore: Send + Sync {
    // DEBUG functions
    fn create_fake_user(&mut self);
    fn create_fake_party(&mut self);
    fn create_fake_invitation(&mut self);
}

#[derive(Default)]
pub struct MockStore {
    pub users: HashMap<UserId, User>,
    pub parties: HashMap<PartyId, Party>,
    pub invitations: HashMap<InvitationToken, Invitation>,
}

#[async_trait]
impl UserPersistence for MockStore {
    async fn create_user(&mut self, user: User) -> anyhow::Result<()> {
        // enforce unique email
        if self.users.values().any(|u| u.email == user.email) {
            bail!("email already exists");
        }

        self.users.insert(user.id, user);
        Ok(())
    }

    async fn get_user_by_id(&self, id: UserId) -> Option<&User> {
        self.users.get(&id)
    }

    async fn get_user_by_email(&self, email: &str) -> Option<&User> {
        self.users.values().find(|u| u.email == email)
    }

    async fn delete_user(&mut self, id: UserId) -> anyhow::Result<()> {
        self.users
            .remove(&id)
            .ok_or_else(|| anyhow!("User could not be removed, as it didn't exist"))?;
        Ok(())
    }
}

#[async_trait]
impl PartyPersistence for MockStore {
    async fn create_party(&mut self, party: Party) -> anyhow::Result<()> {
        if self.parties.contains_key(&party.id) {
            return Err(anyhow!("Unable to create a party : party already exists"));
        }

        self.parties.insert(party.id, party);
        Ok(())
    }

    async fn get_user_role(&self, user_id: UserId, party_id: PartyId) -> anyhow::Result<Role> {
        let party = self.get_party_by_id(party_id).await.ok_or_else(|| {
            anyhow!("Unable to add user ({user_id}) to party ({party_id}) : party does not exist")
        })?;
        party
            .members
            .get(&user_id)
            .ok_or_else(|| anyhow!("Unable to add user ({user_id}) to party ({party_id}) : user does not exist in this party")).cloned()
    }

    async fn is_user_in_party(&self, user_id: UserId, party_id: PartyId) -> anyhow::Result<bool> {
        let party = self.get_party_by_id(party_id).await.ok_or_else(|| {
            anyhow!("Unable to check user ({user_id}) in party ({party_id}) : party does not exist")
        })?;

        Ok(party.members.contains_key(&user_id))
    }

    async fn get_party_by_id(&self, id: PartyId) -> Option<&Party> {
        self.parties.get(&id)
    }
    async fn get_party_by_id_mut(&mut self, id: PartyId) -> Option<&mut Party> {
        self.parties.get_mut(&id)
    }

    async fn add_user_to_party(
        &mut self,
        user_id: UserId,
        party_id: PartyId,
        member_role: Role,
    ) -> anyhow::Result<()> {
        let party = self.get_party_by_id_mut(party_id).await.ok_or_else(|| {
            anyhow!("Unable to add user ({user_id}) to party ({party_id}) : party does not exist")
        })?;

        if party.members.contains_key(&user_id) {
            // no-op user already in party
            return Ok(());
        }

        party.members.insert(user_id, member_role);

        Ok(())
    }

    async fn remove_user_from_party(
        &mut self,
        user_id: UserId,
        party_id: PartyId,
    ) -> anyhow::Result<()> {
        let party = self.get_party_by_id_mut(party_id).await.ok_or_else(|| {
            anyhow!(
                "Unable to remove user ({user_id}) from party ({party_id}) : party does not exist"
            )
        })?;

        if party.members.remove(&user_id).is_some() {
            Ok(())
        } else {
            Err(anyhow!(
                "Unable to remove user ({user_id}) from party ({party_id}) : no such user in party"
            ))
        }
    }

    async fn delete_party(&mut self, party_id: PartyId) -> anyhow::Result<Party> {
        self.parties
            .remove(&party_id)
            .ok_or_else(|| anyhow!("Unable to delete party ({party_id}) : no such party"))
    }
}

#[async_trait]
impl InvitationPersistence for MockStore {
    async fn create_invitation(
        &mut self,
        party_id: PartyId,
        ttl: Duration,
    ) -> anyhow::Result<InvitationToken> {
        // check if party exists
        let _ = self.get_user_by_id(party_id).await.ok_or_else(|| {
            anyhow!("Unable to create an invitation for party ({party_id}) : party does not exist")
        })?;

        let invitation = Invitation::new(ttl, party_id);
        self.invitations
            .insert(invitation.token, invitation.clone());

        Ok(invitation.token)
    }

    async fn consume_invitation(&mut self, token: InvitationToken) -> anyhow::Result<PartyId> {
        let invitation = self
            .invitations
            .remove(&token)
            .ok_or_else(|| anyhow!("Invalid invitation token"))?;

        if !invitation.is_valid() {
            bail!("Invitation expired");
        }

        Ok(invitation.party_id)
    }
}

#[async_trait]
impl DebugStore for MockStore {
    fn create_fake_user(&mut self) {
        let id = UserId::new_v4();
        let password_hash = {
            let salt = SaltString::generate(&mut OsRng);
            let argon2 = Argon2::default();
            argon2
                .hash_password("Louvre".as_bytes(), &salt)
                .unwrap()
                .to_string()
        };

        self.users.insert(
            id,
            crate::user::User {
                id,
                email: format!("user{}@test.dev", self.users.len() + 1),
                username: format!("user{}", self.users.len() + 1),
                created_at: Instant::now(),
                password_hash,
            },
        );
    }

    fn create_fake_party(&mut self) {
        let Some(user) = self.users.values().next() else {
            return;
        };

        let party_id = PartyId::new_v4();
        let mut members = HashMap::new();
        members.insert(user.id, Role::Creator);

        self.parties.insert(
            party_id,
            crate::party::Party {
                id: party_id,
                creator: user.clone(),
                members,
            },
        );
    }

    fn create_fake_invitation(&mut self) {
        let Some(party) = self.parties.values().next() else {
            return;
        };

        let token = InvitationToken::new_v4();
        self.invitations.insert(
            token,
            crate::invitation::Invitation {
                token,
                party_id: party.id,
                valid_until: Instant::now() + Duration::from_secs(3600),
            },
        );
    }
}
