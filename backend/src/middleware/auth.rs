use axum::{
    body::Body,
    extract::State,
    http::{header, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use crate::error::AppError;
use crate::services::session::session_token_from_cookie;
use crate::state::AppState;

pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let config = state.config.get().await;

    if !config.auth.enabled {
        return Ok(next.run(req).await);
    }

    let token = req
        .headers()
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(session_token_from_cookie);

    let Some(token) = token else {
        return Ok(unauthorized_json());
    };

    if state.sessions.validate(&token).await.is_some() {
        Ok(next.run(req).await)
    } else {
        Ok(unauthorized_json())
    }
}

fn unauthorized_json() -> Response {
    AppError::Unauthorized("Authentification requise".into()).into_response()
}
