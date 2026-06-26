use crate::config::{HomeAssistantConfig, HomeAssistantEntityRef};
use crate::error::{AppError, AppResult};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize)]
pub struct HomeAssistantWidgetState {
    pub switchs: Vec<HaSwitchState>,
    pub sensors: Vec<HaSensorState>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HaSwitchState {
    pub entity_id: String,
    pub label: String,
    pub on: bool,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct HaSensorState {
    pub entity_id: String,
    pub label: String,
    pub value: String,
    pub unit: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HaStateResponse {
    #[allow(dead_code)]
    entity_id: String,
    state: String,
    attributes: HaAttributes,
}

#[derive(Debug, Default, Deserialize)]
struct HaAttributes {
    friendly_name: Option<String>,
    unit_of_measurement: Option<String>,
}

struct CachedWidgetState {
    state: HomeAssistantWidgetState,
    fetched_at: Instant,
}

pub struct HomeAssistantService {
    client: Client,
    insecure_client: Client,
    cache: Arc<RwLock<HashMap<String, CachedWidgetState>>>,
}

impl HomeAssistantService {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("GopDash/0.1 (homeassistant)")
            .build()
            .unwrap_or_default();

        let insecure_client = Client::builder()
            .user_agent("GopDash/0.1 (homeassistant)")
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap_or_default();

        Self {
            client,
            insecure_client,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_widget_state(
        &self,
        cfg: &HomeAssistantConfig,
        widget_id: &str,
        switchs: &[HomeAssistantEntityRef],
        sensors: &[HomeAssistantEntityRef],
        cache_secs: u64,
        force: bool,
    ) -> AppResult<HomeAssistantWidgetState> {
        if !force {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(widget_id) {
                if cached.fetched_at.elapsed() < Duration::from_secs(cache_secs) {
                    return Ok(cached.state.clone());
                }
            }
        }

        let state = self
            .fetch_widget_state(cfg, switchs, sensors)
            .await?;

        self.cache.write().await.insert(
            widget_id.to_string(),
            CachedWidgetState {
                state: state.clone(),
                fetched_at: Instant::now(),
            },
        );

        Ok(state)
    }

    pub async fn set_switch(
        &self,
        cfg: &HomeAssistantConfig,
        widget_id: &str,
        item: &HomeAssistantEntityRef,
        on: bool,
    ) -> AppResult<HaSwitchState> {
        let entity_id = &item.entity_id;
        let domain = entity_domain(entity_id)
            .ok_or_else(|| AppError::BadRequest("Invalid entity_id".into()))?;

        let service = if on { "turn_on" } else { "turn_off" };
        let url = format!(
            "{}/api/services/{}/{}",
            base_url(cfg),
            domain,
            service
        );

        let body = serde_json::json!({ "entity_id": entity_id });
        let response = self
            .http(cfg)
            .post(url)
            .bearer_auth(cfg.access_token.trim())
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::BadRequest(format!("Home Assistant unreachable: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::BadRequest(format!(
                "Home Assistant error ({status}): {body}"
            )));
        }

        self.cache.write().await.remove(widget_id);

        self.wait_for_switch_state(cfg, item, on).await
    }

    async fn fetch_widget_state(
        &self,
        cfg: &HomeAssistantConfig,
        switchs: &[HomeAssistantEntityRef],
        sensors: &[HomeAssistantEntityRef],
    ) -> AppResult<HomeAssistantWidgetState> {
        let mut switch_states = Vec::with_capacity(switchs.len());
        for item in switchs {
            switch_states.push(self.fetch_switch(cfg, item).await?);
        }

        let mut sensor_states = Vec::with_capacity(sensors.len());
        for item in sensors {
            sensor_states.push(self.fetch_sensor(cfg, item).await?);
        }

        Ok(HomeAssistantWidgetState {
            switchs: switch_states,
            sensors: sensor_states,
        })
    }

    async fn wait_for_switch_state(
        &self,
        cfg: &HomeAssistantConfig,
        item: &HomeAssistantEntityRef,
        expected_on: bool,
    ) -> AppResult<HaSwitchState> {
        let mut last = None;

        for attempt in 0..6 {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_millis(200)).await;
            }

            let switch_state = self.fetch_switch(cfg, item).await?;
            if switch_state.on == expected_on {
                return Ok(switch_state);
            }
            last = Some(switch_state);
        }

        Ok(last.unwrap_or_else(|| HaSwitchState {
            entity_id: item.entity_id.clone(),
            label: item
                .label
                .clone()
                .filter(|l| !l.trim().is_empty())
                .unwrap_or_else(|| item.entity_id.clone()),
            on: expected_on,
            available: true,
        }))
    }

    async fn fetch_switch(
        &self,
        cfg: &HomeAssistantConfig,
        item: &HomeAssistantEntityRef,
    ) -> AppResult<HaSwitchState> {
        let ha = self.fetch_state(cfg, &item.entity_id).await?;
        let state = ha.state.trim();
        let available = !matches!(
            state.to_ascii_lowercase().as_str(),
            "unavailable" | "unknown"
        );
        let on = is_entity_on(state);

        Ok(HaSwitchState {
            entity_id: item.entity_id.clone(),
            label: label_for(item, &ha),
            on,
            available,
        })
    }

    async fn fetch_sensor(
        &self,
        cfg: &HomeAssistantConfig,
        item: &HomeAssistantEntityRef,
    ) -> AppResult<HaSensorState> {
        let ha = self.fetch_state(cfg, &item.entity_id).await?;

        Ok(HaSensorState {
            entity_id: item.entity_id.clone(),
            label: label_for(item, &ha),
            value: ha.state,
            unit: ha.attributes.unit_of_measurement.clone(),
        })
    }

    async fn fetch_state(
        &self,
        cfg: &HomeAssistantConfig,
        entity_id: &str,
    ) -> AppResult<HaStateResponse> {
        let url = format!("{}/api/states/{}", base_url(cfg), entity_id);
        let response = self
            .http(cfg)
            .get(url)
            .bearer_auth(cfg.access_token.trim())
            .send()
            .await
            .map_err(|e| AppError::BadRequest(format!("Home Assistant unreachable: {e}")))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::BadRequest(format!(
                "Entity not found: {entity_id}"
            )));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::BadRequest(format!(
                "Home Assistant error ({status}): {body}"
            )));
        }

        response
            .json::<HaStateResponse>()
            .await
            .map_err(|e| AppError::BadRequest(format!("Invalid Home Assistant response: {e}")))
    }

    fn http(&self, cfg: &HomeAssistantConfig) -> &Client {
        if cfg.insecure {
            &self.insecure_client
        } else {
            &self.client
        }
    }
}

impl Default for HomeAssistantService {
    fn default() -> Self {
        Self::new()
    }
}

fn base_url(cfg: &HomeAssistantConfig) -> String {
    cfg.url.trim().trim_end_matches('/').to_string()
}

fn entity_domain(entity_id: &str) -> Option<&str> {
    entity_id.split('.').next().filter(|d| !d.is_empty())
}

fn label_for(item: &HomeAssistantEntityRef, ha: &HaStateResponse) -> String {
    item.label
        .clone()
        .filter(|l| !l.trim().is_empty())
        .or_else(|| ha.attributes.friendly_name.clone())
        .unwrap_or_else(|| item.entity_id.clone())
}

fn is_entity_on(state: &str) -> bool {
    matches!(
        state.trim().to_ascii_lowercase().as_str(),
        "on" | "true" | "open" | "opening" | "playing" | "home" | "active"
    )
}
