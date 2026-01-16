use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};

use crate::http::jwt;

pub async fn jwt_auth(req: Request, next: Next) -> Result<Response, StatusCode> {
    let headers = req.headers();

    if let Some(auth_header) = headers.get("Authorization")
        && let Ok(auth_str) = auth_header.to_str()
        && let Some(token) = auth_str.strip_prefix("Bearer ")
        && jwt::validate_jwt(token).is_ok()
    {
        return Ok(next.run(req).await);
    }

    Err(StatusCode::UNAUTHORIZED)
}
