use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::config::{
    AppConfig, SearchEngineConfig, ThemeConfig, ThemeMode, ThemePreset, WidgetConfig,
    WidgetLayoutUpdate,
};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(health))
        .route("/config", get(get_config))
        .route("/config/layout", put(update_layout))
        .route("/docker/containers", get(list_containers))
        .route("/docker/containers/{id}/start", post(start_container))
        .route("/docker/containers/{id}/stop", post(stop_container))
        .route("/docker/containers/{id}/restart", post(restart_container))
        .route("/docker/updates", get(list_docker_updates))
        .route("/docker/containers/{id}/update", post(update_container_image))
        .route("/docker/images/prune", post(prune_unused_images))
        .route("/system", get(system_metrics))
        .route("/weather", get(weather))
        .route("/bookmarks", get(bookmarks))
        .route("/bookmarks/health", get(bookmarks_health))
        .route("/rss/{feed}", get(rss_feed))
        .route("/jellyfin/status", get(jellyfin_status))
        .route("/jellyfin/images/{item_id}", get(jellyfin_image))
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn get_config(State(state): State<Arc<AppState>>) -> AppResult<Json<PublicConfig>> {
    let config = state.config.get().await;
    Ok(Json(PublicConfig::from(&config)))
}

#[derive(Serialize)]
struct PublicConfig {
    title: String,
    refresh_interval: u64,
    widgets: Vec<WidgetConfig>,
    auth_enabled: bool,
    weather_configured: bool,
    weather_show_forecast: bool,
    bookmark_groups: Vec<String>,
    rss_feeds: Vec<String>,
    search_engines: Vec<SearchEngineConfig>,
    jellyfin_configured: bool,
    theme: PublicTheme,
    persist_layout: bool,
    locale: String,
    timezone: String,
}

#[derive(Serialize)]
struct PublicTheme {
    mode: String,
    preset: String,
}

impl From<&AppConfig> for PublicConfig {
    fn from(c: &AppConfig) -> Self {
        Self {
            title: c.title.clone(),
            refresh_interval: c.refresh_interval,
            widgets: c.widgets.clone(),
            auth_enabled: c.auth.enabled,
            weather_configured: c.weather.is_some(),
            weather_show_forecast: c
                .weather
                .as_ref()
                .map(|w| w.show_forecast)
                .unwrap_or(true),
            bookmark_groups: c.bookmarks.iter().map(|b| b.name.clone()).collect(),
            rss_feeds: c.rss.iter().map(|r| r.name.clone()).collect(),
            search_engines: c.search_engines.clone(),
            jellyfin_configured: c.jellyfin.as_ref().is_some_and(|j| {
                !j.url.trim().is_empty() && !j.api_key.trim().is_empty()
            }),
            theme: PublicTheme::from(&c.theme),
            persist_layout: c.settings.persist_layout,
            locale: c.settings.locale.clone(),
            timezone: c.settings.timezone.clone(),
        }
    }
}

impl From<&ThemeConfig> for PublicTheme {
    fn from(t: &ThemeConfig) -> Self {
        Self {
            mode: theme_mode_str(t.mode).into(),
            preset: theme_preset_str(t.preset).into(),
        }
    }
}

fn theme_mode_str(mode: ThemeMode) -> &'static str {
    match mode {
        ThemeMode::Light => "light",
        ThemeMode::Dark => "dark",
        ThemeMode::System => "system",
    }
}

fn theme_preset_str(preset: ThemePreset) -> &'static str {
    match preset {
        ThemePreset::Default => "default",
        ThemePreset::Ocean => "ocean",
        ThemePreset::Forest => "forest",
        ThemePreset::Rose => "rose",
    }
}

#[derive(Deserialize)]
struct LayoutUpdate {
    widgets: Vec<WidgetLayoutUpdate>,
}

async fn update_layout(
    State(state): State<Arc<AppState>>,
    Json(layout): Json<LayoutUpdate>,
) -> AppResult<Json<serde_json::Value>> {
    let config = state.config.get().await;

    if !config.settings.persist_layout {
        return Ok(Json(serde_json::json!({
            "status": "ok",
            "persisted": false,
            "message": "Layout persistence disabled in config (settings.persist_layout)"
        })));
    }

    state
        .config
        .update_layouts(&layout.widgets)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "persisted": true
    })))
}

#[derive(Deserialize)]
struct ContainerQuery {
    filter: Option<String>,
    show_all: Option<bool>,
}

async fn list_containers(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ContainerQuery>,
) -> AppResult<Json<Vec<crate::services::docker::ContainerInfo>>> {
    let config = state.config.get().await;
    let filter_names: Vec<String> = q
        .filter
        .map(|f| f.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let show_all = q.show_all.unwrap_or_else(|| {
        config
            .widgets
            .iter()
            .any(|w| matches!(w, WidgetConfig::Docker { show_all: true, .. }))
    });

    let containers = state
        .docker
        .list_containers(&filter_names, show_all)
        .await?;
    Ok(Json(containers))
}

async fn start_container(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    state.docker.start_container(&id).await?;
    Ok(Json(serde_json::json!({ "status": "started" })))
}

async fn stop_container(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    state.docker.stop_container(&id).await?;
    Ok(Json(serde_json::json!({ "status": "stopped" })))
}

async fn restart_container(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    state.docker.restart_container(&id).await?;
    Ok(Json(serde_json::json!({ "status": "restarted" })))
}

async fn list_docker_updates(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ContainerQuery>,
) -> AppResult<Json<Vec<crate::services::docker_updates::ContainerUpdateInfo>>> {
    let config = state.config.get().await;
    let filter_names: Vec<String> = q
        .filter
        .map(|f| f.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let show_all = q.show_all.unwrap_or_else(|| {
        config.widgets.iter().any(|w| {
            matches!(
                w,
                WidgetConfig::Docker { show_all: true, .. }
                    | WidgetConfig::DockerUpdates { show_all: true, .. }
            )
        })
    });

    let updates = state.docker.list_updates(&filter_names, show_all).await?;
    Ok(Json(updates))
}

async fn update_container_image(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    state.docker.update_container_image(&id).await?;
    Ok(Json(serde_json::json!({ "status": "updated" })))
}

async fn prune_unused_images(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<crate::services::docker::ImagePruneResult>> {
    let result = state.docker.prune_unused_images().await?;
    Ok(Json(result))
}

async fn system_metrics(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<crate::services::system::SystemMetrics>> {
    Ok(Json(state.system.collect().await))
}

#[derive(Deserialize)]
struct WeatherQuery {
    location: Option<String>,
}

async fn weather(
    State(state): State<Arc<AppState>>,
    Query(q): Query<WeatherQuery>,
) -> AppResult<Json<crate::services::weather::WeatherData>> {
    let config = state.config.get().await;
    let weather_cfg = config
        .weather
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("Weather not configured".into()))?;

    let location = q
        .location
        .unwrap_or_else(|| weather_cfg.default_location.clone());

    let data = state
        .weather
        .get_weather(
            &location,
            &weather_cfg.units,
            &config.settings.locale,
            &config.settings.timezone,
        )
        .await?;
    Ok(Json(data))
}

#[derive(Deserialize)]
struct BookmarkQuery {
    group: Option<String>,
}

async fn bookmarks(
    State(state): State<Arc<AppState>>,
    Query(q): Query<BookmarkQuery>,
) -> AppResult<Json<Vec<crate::config::BookmarkGroup>>> {
    let config = state.config.get().await;
    let groups = if let Some(name) = q.group {
        config
            .bookmarks
            .into_iter()
            .filter(|g| g.name == name)
            .collect()
    } else {
        config.bookmarks
    };
    Ok(Json(groups))
}

async fn bookmarks_health(
    State(state): State<Arc<AppState>>,
    Query(q): Query<BookmarkQuery>,
) -> AppResult<Json<Vec<crate::services::health::BookmarkHealthResult>>> {
    let config = state.config.get().await;
    let groups: Vec<_> = if let Some(ref name) = q.group {
        config.bookmarks.iter().filter(|g| &g.name == name).collect()
    } else {
        config.bookmarks.iter().collect()
    };

    let links: Vec<(&str, &crate::config::BookmarkLink)> = groups
        .iter()
        .flat_map(|g| g.links.iter().map(|l| (g.name.as_str(), l)))
        .collect();

    let cache_secs = config.refresh_interval.max(15);
    let results = state.health.check_bookmarks(&links, cache_secs).await;

    Ok(Json(results))
}

async fn rss_feed(
    State(state): State<Arc<AppState>>,
    Path(feed): Path<String>,
) -> AppResult<Json<crate::services::rss::RssFeedData>> {
    let config = state.config.get().await;
    let feed_cfg = config
        .rss
        .iter()
        .find(|f| f.name == feed)
        .ok_or_else(|| AppError::NotFound(format!("RSS feed not found: {feed}")))?;

    let max_items = config
        .widgets
        .iter()
        .find_map(|w| {
            if let WidgetConfig::Rss { feed: f, max_items, .. } = w {
                if f == &feed {
                    Some(*max_items)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .unwrap_or(10);

    let data = state
        .rss
        .get_feed(&feed_cfg.name, &feed_cfg.url, max_items, feed_cfg.refresh_minutes)
        .await?;
    Ok(Json(data))
}

#[derive(Deserialize)]
struct JellyfinStatusQuery {
    #[serde(default = "default_true")]
    show_now_playing: bool,
    #[serde(default = "default_true")]
    show_library_counts: bool,
    #[serde(default = "default_max_sessions_query")]
    max_sessions: u32,
}

fn default_true() -> bool {
    true
}

fn default_max_sessions_query() -> u32 {
    3
}

async fn jellyfin_status(
    State(state): State<Arc<AppState>>,
    Query(q): Query<JellyfinStatusQuery>,
) -> AppResult<Json<crate::services::jellyfin::JellyfinStatus>> {
    let config = state.config.get().await;
    let jellyfin_cfg = config
        .jellyfin
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("Jellyfin not configured".into()))?;

    let cache_secs = config.refresh_interval.max(5);
    let status = state
        .jellyfin
        .get_status(
            jellyfin_cfg,
            q.show_now_playing,
            q.show_library_counts,
            q.max_sessions.max(1),
            cache_secs,
        )
        .await?;

    Ok(Json(status))
}

async fn jellyfin_image(
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<String>,
) -> AppResult<Response> {
    let config = state.config.get().await;
    let jellyfin_cfg = config
        .jellyfin
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("Jellyfin not configured".into()))?;

    let (bytes, content_type) = state.jellyfin.get_image(jellyfin_cfg, &item_id).await?;

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, content_type)],
        bytes,
    )
        .into_response())
}
