use crate::config::BookmarkLink;
use reqwest::Client;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Up,
    Down,
}

#[derive(Debug, Clone, Serialize)]
pub struct BookmarkHealthResult {
    pub name: String,
    pub url: String,
    pub status: HealthStatus,
    pub status_code: Option<u16>,
    pub latency_ms: u64,
    pub error: Option<String>,
}

struct HealthProbe {
    cache_id: String,
    url: String,
    method: String,
    timeout_secs: u64,
    expected_status: Option<u16>,
    insecure: bool,
}

struct CachedCheck {
    result: BookmarkHealthResult,
    fetched_at: Instant,
}

pub struct HealthService {
    client: Client,
    insecure_client: Client,
    cache: Arc<RwLock<HashMap<String, CachedCheck>>>,
}

impl HealthService {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("GopDash/0.1 (health-check)")
            .cookie_store(true)
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .unwrap_or_default();

        let insecure_client = Client::builder()
            .user_agent("GopDash/0.1 (health-check)")
            .danger_accept_invalid_certs(true)
            .cookie_store(true)
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .unwrap_or_default();

        Self {
            client,
            insecure_client,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn check_bookmarks(
        &self,
        links: &[(&str, &BookmarkLink)],
        cache_secs: u64,
    ) -> Vec<BookmarkHealthResult> {
        let mut results = Vec::new();
        for (group, link) in links {
            if let Some(probe) = probe_for_link(group, link) {
                results.push(self.check_one(&probe, link, cache_secs).await);
            }
        }
        results
    }

    async fn check_one(
        &self,
        probe: &HealthProbe,
        link: &BookmarkLink,
        cache_secs: u64,
    ) -> BookmarkHealthResult {
        if cache_secs > 0 {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&probe.cache_id) {
                if cached.fetched_at.elapsed().as_secs() < cache_secs {
                    return cached.result.clone();
                }
            }
        }

        let result = self.probe(probe, link).await;

        if cache_secs > 0 {
            self.cache.write().await.insert(
                probe.cache_id.clone(),
                CachedCheck {
                    result: result.clone(),
                    fetched_at: Instant::now(),
                },
            );
        }

        result
    }

    async fn probe(&self, probe: &HealthProbe, link: &BookmarkLink) -> BookmarkHealthResult {
        let base = BookmarkHealthResult {
            name: link.name.clone(),
            url: link.url.clone(),
            status: HealthStatus::Down,
            status_code: None,
            latency_ms: 0,
            error: None,
        };

        let client = if probe.insecure {
            &self.insecure_client
        } else {
            &self.client
        };

        let timeout = Duration::from_secs(probe.timeout_secs.max(1));
        let started = Instant::now();

        let request = match probe.method.to_uppercase().as_str() {
            "HEAD" => client.head(&probe.url),
            _ => client.get(&probe.url),
        };

        match request.timeout(timeout).send().await {
            Ok(response) => {
                let status_code = response.status().as_u16();
                let latency_ms = started.elapsed().as_millis() as u64;
                let up = response.status().is_success()
                    || probe
                        .expected_status
                        .is_some_and(|expected| expected == status_code);

                BookmarkHealthResult {
                    status: if up {
                        HealthStatus::Up
                    } else {
                        HealthStatus::Down
                    },
                    status_code: Some(status_code),
                    latency_ms,
                    error: if up {
                        None
                    } else {
                        Some(format!("HTTP {status_code}"))
                    },
                    ..base
                }
            }
            Err(err) => BookmarkHealthResult {
                latency_ms: started.elapsed().as_millis() as u64,
                error: Some(err.to_string()),
                ..base
            },
        }
    }
}

fn probe_for_link(group: &str, link: &BookmarkLink) -> Option<HealthProbe> {
    if !link.health_check {
        return None;
    }

    Some(HealthProbe {
        cache_id: format!("{group}:{}", link.name),
        url: link
            .health_url
            .clone()
            .unwrap_or_else(|| link.url.clone()),
        method: link.method.clone(),
        timeout_secs: link.timeout_secs,
        expected_status: link.expected_status,
        insecure: link.insecure,
    })
}

impl Default for HealthService {
    fn default() -> Self {
        Self::new()
    }
}
