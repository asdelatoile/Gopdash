use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

pub const SESSION_COOKIE: &str = "gopdash_session";
const SESSION_TTL: Duration = Duration::from_secs(7 * 24 * 3600);

struct Session {
    username: String,
    expires_at: Instant,
}

pub struct SessionStore {
    sessions: RwLock<HashMap<String, Session>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    pub async fn create(&self, username: String) -> String {
        self.purge_expired().await;
        let token = Uuid::new_v4().to_string();
        self.sessions.write().await.insert(
            token.clone(),
            Session {
                username,
                expires_at: Instant::now() + SESSION_TTL,
            },
        );
        token
    }

    pub async fn validate(&self, token: &str) -> Option<String> {
        self.purge_expired().await;
        let sessions = self.sessions.read().await;
        sessions.get(token).and_then(|s| {
            if s.expires_at > Instant::now() {
                Some(s.username.clone())
            } else {
                None
            }
        })
    }

    pub async fn revoke(&self, token: &str) {
        self.sessions.write().await.remove(token);
    }

    async fn purge_expired(&self) {
        let now = Instant::now();
        self.sessions
            .write()
            .await
            .retain(|_, s| s.expires_at > now);
    }
}

pub fn session_cookie_value(token: &str) -> String {
    format!(
        "{SESSION_COOKIE}={token}; HttpOnly; Path=/; SameSite=Lax; Max-Age={}",
        SESSION_TTL.as_secs()
    )
}

pub fn clear_session_cookie() -> String {
    format!("{SESSION_COOKIE}=; HttpOnly; Path=/; SameSite=Lax; Max-Age=0")
}

pub fn session_token_from_cookie(cookie_header: &str) -> Option<String> {
    cookie_header.split(';').find_map(|part| {
        let (key, value) = part.trim().split_once('=')?;
        if key == SESSION_COOKIE {
            Some(value.to_string())
        } else {
            None
        }
    })
}
