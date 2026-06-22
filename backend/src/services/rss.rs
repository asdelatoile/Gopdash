use crate::error::{AppError, AppResult};
use reqwest::Client;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize)]
pub struct RssItem {
    pub title: String,
    pub link: String,
    pub pub_date: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RssFeedData {
    pub name: String,
    pub items: Vec<RssItem>,
}

struct CachedFeed {
    data: RssFeedData,
    fetched_at: std::time::Instant,
    ttl_secs: u64,
}

pub struct RssService {
    client: Client,
    cache: Arc<RwLock<HashMap<String, CachedFeed>>>,
}

impl RssService {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (compatible; Gopdash/0.1; RSS Reader)")
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_default();

        Self {
            client,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_feed(
        &self,
        name: &str,
        url: &str,
        max_items: u32,
        refresh_minutes: u64,
    ) -> AppResult<RssFeedData> {
        let cache_key = url.to_string();
        let ttl_secs = refresh_minutes * 60;

        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                if cached.fetched_at.elapsed().as_secs() < cached.ttl_secs {
                    return Ok(cached.data.clone());
                }
            }
        }

        let data = self.fetch_feed(name, url, max_items).await?;

        self.cache.write().await.insert(
            cache_key,
            CachedFeed {
                data: data.clone(),
                fetched_at: std::time::Instant::now(),
                ttl_secs,
            },
        );

        Ok(data)
    }

    async fn fetch_feed(&self, name: &str, url: &str, max_items: u32) -> AppResult<RssFeedData> {
        let response = self
            .client
            .get(url)
            .header("Accept", "application/rss+xml, application/xml, text/xml, */*")
            .send()
            .await
            .map_err(|e| AppError::Internal(e.into()))?;

        if !response.status().is_success() {
            return Err(AppError::BadRequest(format!(
                "RSS feed returned HTTP {}",
                response.status()
            )));
        }

        let content = response
            .bytes()
            .await
            .map_err(|e| AppError::Internal(e.into()))?;

        let channel = rss::Channel::read_from(&content[..]).map_err(|e| {
            AppError::BadRequest(format!(
                "Invalid RSS feed: {e}. The URL may require a different format or block automated access."
            ))
        })?;

        let items = channel
            .items()
            .iter()
            .take(max_items as usize)
            .map(|item| RssItem {
                title: item.title().unwrap_or("Untitled").to_string(),
                link: item.link().unwrap_or("").to_string(),
                pub_date: item.pub_date().map(|d| d.to_string()),
                description: item
                    .description()
                    .map(|d| strip_html(d))
                    .filter(|d| !d.is_empty()),
            })
            .collect();

        Ok(RssFeedData {
            name: name.into(),
            items,
        })
    }
}

fn strip_html(input: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    if result.len() > 200 {
        result.truncate(200);
        result.push('…');
    }
    result
}
