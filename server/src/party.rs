use std::collections::HashMap;

use uuid::Uuid;

use crate::user::{User, UserId};

pub type PartyId = Uuid;

#[derive(Clone, Debug, PartialEq)]
pub struct Party {
    pub id: PartyId,
    creator: User,
    /// Users that have joined this party
    pub members: HashMap<UserId, Role>,
}

impl Party {
    pub fn create(sender: &User) -> Self {
        // The only starting member of a party is it's creator
        let mut members: HashMap<UserId, Role> = HashMap::new();
        members.insert(sender.id, Role::Creator);

        Self {
            id: Uuid::new_v4(),
            creator: sender.clone(),
            members,
        }
    }

    // ///Send a Message to everyone in the party including the sender
    // pub fn broadcast_inclusive(&self, message: ServerMessage) -> anyhow::Result<()> {
    //     let message = Arc::new(message);
    //     for member in &self.members {
    //         member.tx_channel.send(message.clone())?
    //     }
    //     Ok(())
    // }

    // /// Send a Message to everyone in the party excluding the sender
    // pub fn broadcast_exclusive(&self, message: ServerMessage) -> anyhow::Result<()> {
    //     let message = Arc::new(message);
    //     for member in &self.members {
    //         if member.id != message.sender.id {
    //             member.tx_channel.send(message.clone())?
    //         }
    //     }
    //     Ok(())
    // }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Role {
    Creator,
    Admin,
    Default,
}
