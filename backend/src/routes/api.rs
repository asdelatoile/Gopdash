use axum::{
    extract::{Path, Query, State},
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
        .route("/system", get(system_metrics))
        .route("/weather", get(weather))
        .route("/bookmarks", get(bookmarks))
        .route("/rss/{feed}", get(rss_feed))
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
