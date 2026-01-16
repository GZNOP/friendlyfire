use std::{
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::anyhow;
use axum::http::HeaderValue;
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::user::UserId;

const JWT_ISSUER: &str = "friendlyfire";
const JWT_AUDIENCE: &str = "friendlyfire-clients";
const JWT_TTL_SECONDS: i64 = 60 * 60 * 8; //8h

/// WARNING : Do not put sensitive data here, this is not encrypted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    ///user ID (string or UUID)
    sub: String,

    ///who issued the token
    iss: String,

    ///intended audience (your service)
    aud: String,

    ///expiration timestamp
    exp: i64,

    ///issued-at timestamp
    iat: i64,

    ///not valid before (optional)
    nbf: i64,

    ///token ID (useful for revocation)
    jti: String,
}

impl Claims {
    pub fn user_id(&self) -> UserId {
        Uuid::from_str(&self.sub).unwrap()
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock went backwards")
        .as_secs() as i64
}

pub fn issue_jwt(user_id: Uuid) -> anyhow::Result<String> {
    let now = unix_now();

    let claims = Claims {
        sub: user_id.to_string(),
        iss: JWT_ISSUER.to_string(),
        aud: JWT_AUDIENCE.to_string(),
        iat: now,
        nbf: now,
        exp: now + JWT_TTL_SECONDS,
        jti: Uuid::new_v4().to_string(),
    };

    let token = encode(
        &Header::new(Algorithm::HS512),
        &claims,
        &EncodingKey::from_secret(&dotenv::var("JWT_SECRET")?.into_bytes()),
    )?;

    Ok(token)
}

pub fn validate_jwt(token: &str) -> anyhow::Result<Claims> {
    let mut validation = Validation::new(Algorithm::HS512);
    validation.set_issuer(&[JWT_ISSUER]);
    validation.set_audience(&[JWT_AUDIENCE]);
    validation.validate_nbf = true;

    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(&dotenv::var("JWT_SECRET")?.into_bytes()),
        &validation,
    )?;

    Ok(data.claims)
}

pub fn decode_header(header: &HeaderValue) -> anyhow::Result<TokenData<Claims>> {
    if let Ok(auth_str) = header.to_str()
        && let Some(token) = auth_str.strip_prefix("Bearer ")
    {
        let key = &DecodingKey::from_secret(&dotenv::var("JWT_SECRET")?.into_bytes());

        let mut validation = Validation::new(Algorithm::HS512);
        validation.set_issuer(&[JWT_ISSUER]);
        validation.set_audience(&[JWT_AUDIENCE]);
        validation.validate_nbf = true;

        return jsonwebtoken::decode::<Claims>(token, key, &validation)
            .map_err(|_err| anyhow!("Unable to decode JWT"));
    }
    Err(anyhow!("Unable to decode JWT"))
}
