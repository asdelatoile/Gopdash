use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::services::session::{clear_session_cookie, session_cookie_value, session_token_from_cookie};
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/auth/session", get(session_status))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
}

#[derive(Serialize)]
struct SessionResponse {
    auth_enabled: bool,
    authenticated: bool,
    username: Option<String>,
}

async fn session_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> AppResult<Json<SessionResponse>> {
    let config = state.config.get().await;

    if !config.auth.enabled {
        return Ok(Json(SessionResponse {
            auth_enabled: false,
            authenticated: true,
            username: None,
        }));
    }

    let username = session_from_headers(&state, &headers).await;

    Ok(Json(SessionResponse {
        auth_enabled: true,
        authenticated: username.is_some(),
        username,
    }))
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct LoginResponse {
    status: &'static str,
    username: String,
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginRequest>,
) -> AppResult<(StatusCode, [(header::HeaderName, HeaderValue); 1], Json<LoginResponse>)> {
    let config = state.config.get().await;

    if !config.auth.enabled {
        return Err(AppError::BadRequest("Authentication is disabled".into()));
    }

    let expected_user = config.auth.username.as_deref().unwrap_or("admin");
    let expected_pass = config.auth.password.as_deref().unwrap_or("");

    if body.username != expected_user || body.password != expected_pass {
        return Err(AppError::Unauthorized("Identifiants invalides".into()));
    }

    let token = state.sessions.create(expected_user.to_string()).await;
    let cookie = HeaderValue::from_str(&session_cookie_value(&token))
        .map_err(|e| AppError::Internal(e.into()))?;

    Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        Json(LoginResponse {
            status: "ok",
            username: expected_user.to_string(),
        }),
    ))
}

async fn logout(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> AppResult<(StatusCode, [(header::HeaderName, HeaderValue); 1], Json<serde_json::Value>)> {
    if let Some(token) = cookie_token(&headers) {
        state.sessions.revoke(&token).await;
    }

    let cookie = HeaderValue::from_str(&clear_session_cookie())
        .map_err(|e| AppError::Internal(e.into()))?;

    Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        Json(serde_json::json!({ "status": "ok" })),
    ))
}

async fn session_from_headers(state: &AppState, headers: &HeaderMap) -> Option<String> {
    let token = cookie_token(headers)?;
    state.sessions.validate(&token).await
}

fn cookie_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(session_token_from_cookie)
}
