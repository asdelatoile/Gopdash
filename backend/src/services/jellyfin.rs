use crate::config::JellyfinConfig;
use crate::error::{AppError, AppResult};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize)]
pub struct JellyfinStatus {
    pub now_playing: Vec<NowPlayingSession>,
    pub library_counts: Option<LibraryCounts>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NowPlayingSession {
    pub user_name: String,
    pub client: String,
    pub device_name: String,
    pub item_id: String,
    pub item_type: String,
    pub title: String,
    pub series_name: Option<String>,
    pub season_episode: Option<String>,
    pub year: Option<i32>,
    pub is_paused: bool,
    pub position_ticks: i64,
    pub runtime_ticks: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LibraryCounts {
    pub movies: i32,
    pub series: i32,
    pub episodes: i32,
    pub music_albums: i32,
    pub music_tracks: i32,
}

struct CachedStatus {
    now_playing: Vec<NowPlayingSession>,
    library_counts: LibraryCounts,
    fetched_at: Instant,
}

pub struct JellyfinService {
    client: Client,
    insecure_client: Client,
    cache: Arc<RwLock<Option<CachedStatus>>>,
}

impl JellyfinService {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("GopDash/0.1 (jellyfin)")
            .build()
            .unwrap_or_default();

        let insecure_client = Client::builder()
            .user_agent("GopDash/0.1 (jellyfin)")
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap_or_default();

        Self {
            client,
            insecure_client,
            cache: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn get_status(
        &self,
        cfg: &JellyfinConfig,
        show_now_playing: bool,
        show_library_counts: bool,
        max_sessions: u32,
        cache_secs: u64,
    ) -> AppResult<JellyfinStatus> {
        let needs_fetch = {
            let cache = self.cache.read().await;
            cache
                .as_ref()
                .is_none_or(|c| c.fetched_at.elapsed().as_secs() >= cache_secs)
        };

        if needs_fetch {
            let (sessions, counts) = tokio::join!(
                self.fetch_sessions(cfg),
                self.fetch_counts(cfg),
            );
            let now_playing = sessions?;
            let library_counts = counts?;

            *self.cache.write().await = Some(CachedStatus {
                now_playing,
                library_counts,
                fetched_at: Instant::now(),
            });
        }

        let cache = self.cache.read().await;
        let cached = cache
            .as_ref()
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Jellyfin cache empty")))?;

        Ok(JellyfinStatus {
            now_playing: if show_now_playing {
                cached
                    .now_playing
                    .iter()
                    .take(max_sessions as usize)
                    .cloned()
                    .collect()
            } else {
                vec![]
            },
            library_counts: if show_library_counts {
                Some(cached.library_counts.clone())
            } else {
                None
            },
        })
    }

    pub async fn get_image(
        &self,
        cfg: &JellyfinConfig,
        item_id: &str,
    ) -> AppResult<(Vec<u8>, String)> {
        let base = normalize_base_url(&cfg.url);
        let url = format!(
            "{base}/Items/{item_id}/Images/Primary?maxHeight=300&quality=90"
        );

        let response = self
            .http(cfg.insecure)
            .get(&url)
            .header("Authorization", auth_header(&cfg.api_key))
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Jellyfin image request failed: {e}")))?;

        if !response.status().is_success() {
            return Err(AppError::NotFound(format!(
                "Jellyfin image not found for item {item_id}"
            )));
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/jpeg")
            .to_string();

        let bytes = response
            .bytes()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Jellyfin image read failed: {e}")))?;

        Ok((bytes.to_vec(), content_type))
    }

    fn http(&self, insecure: bool) -> &Client {
        if insecure {
            &self.insecure_client
        } else {
            &self.client
        }
    }

    async fn fetch_sessions(&self, cfg: &JellyfinConfig) -> AppResult<Vec<NowPlayingSession>> {
        let base = normalize_base_url(&cfg.url);
        let url = format!("{base}/Sessions");

        let sessions: Vec<JellyfinSession> = self
            .http(cfg.insecure)
            .get(&url)
            .header("Authorization", auth_header(&cfg.api_key))
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Jellyfin sessions request failed: {e}")))?
            .error_for_status()
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Jellyfin sessions error: {e}")))?
            .json()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Jellyfin sessions parse failed: {e}")))?;

        Ok(sessions
            .into_iter()
            .filter_map(|s| session_to_now_playing(s))
            .collect())
    }

    async fn fetch_counts(&self, cfg: &JellyfinConfig) -> AppResult<LibraryCounts> {
        let base = normalize_base_url(&cfg.url);
        let url = format!("{base}/Items/Counts");

        let raw: JellyfinCountsRaw = self
            .http(cfg.insecure)
            .get(&url)
            .header("Authorization", auth_header(&cfg.api_key))
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Jellyfin counts request failed: {e}")))?
            .error_for_status()
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Jellyfin counts error: {e}")))?
            .json()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Jellyfin counts parse failed: {e}")))?;

        Ok(LibraryCounts {
            movies: raw.movie_count,
            series: raw.series_count,
            episodes: raw.episode_count,
            music_albums: raw.album_count,
            music_tracks: raw.song_count,
        })
    }
}

fn auth_header(api_key: &str) -> String {
    format!(
        r#"MediaBrowser Client="GopDash", Device="GopDash", DeviceId="gopdash", Version="0.1.0", Token="{}""#,
        api_key
    )
}

fn normalize_base_url(url: &str) -> String {
    url.trim_end_matches('/').to_string()
}

fn session_to_now_playing(session: JellyfinSession) -> Option<NowPlayingSession> {
    let item = session.now_playing_item?;
    let play_state = session.play_state.unwrap_or(JellyfinPlayState {
        position_ticks: None,
        is_paused: None,
    });

    let season_episode = format_season_episode(item.parent_index_number, item.index_number);

    Some(NowPlayingSession {
        user_name: session.user_name.unwrap_or_else(|| "Unknown".into()),
        client: session.client.unwrap_or_default(),
        device_name: session.device_name.unwrap_or_default(),
        item_id: item.id,
        item_type: item.item_type,
        title: item.name,
        series_name: item.series_name,
        season_episode,
        year: item.production_year,
        is_paused: play_state.is_paused.unwrap_or(false),
        position_ticks: play_state.position_ticks.unwrap_or(0),
        runtime_ticks: item.run_time_ticks.unwrap_or(0),
    })
}

fn format_season_episode(season: Option<i32>, episode: Option<i32>) -> Option<String> {
    match (season, episode) {
        (Some(s), Some(e)) => Some(format!("S{s:02}E{e:02}")),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct JellyfinSession {
    user_name: Option<String>,
    client: Option<String>,
    device_name: Option<String>,
    now_playing_item: Option<JellyfinItem>,
    play_state: Option<JellyfinPlayState>,
}

#[derive(Debug, Deserialize)]
struct JellyfinItem {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Type")]
    item_type: String,
    #[serde(rename = "SeriesName")]
    series_name: Option<String>,
    #[serde(rename = "IndexNumber")]
    index_number: Option<i32>,
    #[serde(rename = "ParentIndexNumber")]
    parent_index_number: Option<i32>,
    #[serde(rename = "ProductionYear")]
    production_year: Option<i32>,
    #[serde(rename = "RunTimeTicks")]
    run_time_ticks: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct JellyfinPlayState {
    position_ticks: Option<i64>,
    is_paused: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct JellyfinCountsRaw {
    movie_count: i32,
    series_count: i32,
    episode_count: i32,
    album_count: i32,
    song_count: i32,
}
