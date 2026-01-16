use std::sync::Arc;

use anyhow::Result;
use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use axum::{Json, extract::State, http::StatusCode};
use log::info;
use serde::{Deserialize, Serialize};

use crate::{
    http::{jwt, state::AppState},
    user::{User, UserPersistence},
};

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
pub struct TokenResponse {
    jwt: String,
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<TokenResponse>, StatusCode> {
    let store = state.store.read().await;
    let user = store
        .get_user_by_email(&payload.email)
        .await
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
pub struct RegisterRequest {
    email: String,
    username: String,
    password: String,
}

pub async fn register(
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

    if store.create_user(user.clone()).await.is_err() {
        return StatusCode::UNAUTHORIZED;
    }

    info!("Successfully registered user: {}", user);

    StatusCode::CREATED
}
