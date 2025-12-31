use std::fmt::Display;

use tokio::time::Instant;
use uuid::Uuid;

pub type UserId = Uuid;
pub type AuthToken = Uuid;

#[derive(Clone, Debug, PartialEq)]
pub struct User {
    pub id: UserId,
    pub email: String,
    username: String,
    pub created_at: Instant,
    pub password_hash: String,
}

impl Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "User {{ id: {}, email: {}, username: {}, created_at: {:?} }}",
            self.id, self.email, self.username, self.created_at
        )
    }
}

impl User {
    pub fn new(email: String, username: String, password_hash: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            email,
            username,
            created_at: Instant::now(),
            password_hash,
        }
    }
}
