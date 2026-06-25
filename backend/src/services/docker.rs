use crate::config::{AppConfig, ConfigManager, WidgetConfig};
use crate::error::{AppError, AppResult};
use bollard::container::{
    ListContainersOptions, MemoryStats, RestartContainerOptions, StartContainerOptions,
    Stats, StatsOptions, StopContainerOptions,
};
use bollard::image::PruneImagesOptions;
use bollard::models::ContainerSummary;
use bollard::Docker;
use futures::StreamExt;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub state: String,
    pub health: Option<String>,
    pub cpu_percent: f64,
    pub memory_usage: u64,
    pub memory_limit: u64,
    pub memory_percent: f64,
    pub compose_project: Option<String>,
    pub compose_service: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImagePruneResult {
    pub images_deleted: usize,
    pub space_reclaimed: u64,
}

pub struct DockerService {
    docker: Option<Docker>,
    stats_cache: Arc<RwLock<HashMap<String, ContainerStats>>>,
    updates: crate::services::docker_updates::DockerUpdatesService,
}

#[derive(Debug, Clone, Default)]
struct ContainerStats {
    cpu_percent: f64,
    memory_usage: u64,
    memory_limit: u64,
    /// Raw counters for delta CPU calculation between polling cycles.
    cpu_total_usage: u64,
    system_cpu_usage: u64,
}

impl DockerService {
    pub async fn new(socket_path: &str) -> anyhow::Result<Self> {
        let path = resolve_docker_socket(socket_path);
        let docker = match Docker::connect_with_unix(&path, 120, bollard::API_DEFAULT_VERSION) {
            Ok(docker) => {
                match docker.ping().await {
                    Ok(_) => tracing::info!("Connected to Docker at {path}"),
                    Err(e) => tracing::warn!("Docker ping failed at {path}: {e}"),
                }
                Some(docker)
            }
            Err(e) => {
                tracing::warn!(
                    "Docker unavailable at {path}: {e}. Le widget Docker sera inactif."
                );
                None
            }
        };

        Ok(Self {
            docker,
            stats_cache: Arc::new(RwLock::new(HashMap::new())),
            updates: crate::services::docker_updates::DockerUpdatesService::new(),
        })
    }

    pub async fn list_updates(
        &self,
        filter_names: &[String],
        show_all: bool,
        force: bool,
        refresh_hours: u64,
    ) -> AppResult<crate::services::docker_updates::DockerUpdatesResponse> {
        let docker = self.docker()?;
        let ttl_secs = refresh_hours.saturating_mul(3600).max(60);
        self.updates
            .get_updates(docker, filter_names, show_all, force, ttl_secs)
            .await
    }

    pub async fn update_container_image(&self, id: &str) -> AppResult<()> {
        let docker = self.docker()?;
        self.updates.update_container(docker, id).await
    }

    pub async fn prune_unused_images(&self) -> AppResult<ImagePruneResult> {
        let docker = self.docker()?;
        let mut filters = HashMap::new();
        filters.insert("dangling".to_string(), vec!["false".to_string()]);

        let response = docker
            .prune_images(Some(PruneImagesOptions {
                filters,
                ..Default::default()
            }))
            .await
            .map_err(|e| AppError::Docker(e.to_string()))?;

        Ok(ImagePruneResult {
            images_deleted: response.images_deleted.map(|v| v.len()).unwrap_or(0),
            space_reclaimed: response.space_reclaimed.unwrap_or(0).max(0) as u64,
        })
    }

    fn docker(&self) -> AppResult<&Docker> {
        self.docker
            .as_ref()
            .ok_or_else(|| AppError::Docker("Docker socket unavailable".into()))
    }

    pub async fn list_containers(
        &self,
        filter_names: &[String],
        show_all: bool,
    ) -> AppResult<Vec<ContainerInfo>> {
        let options = Some(ListContainersOptions::<String> {
            all: show_all,
            ..Default::default()
        });

        let containers = self
            .docker()?
            .list_containers(options)
            .await
            .map_err(|e| AppError::Docker(e.to_string()))?;

        let stats = self.stats_cache.read().await;

        let mut result: Vec<ContainerInfo> = containers
            .into_iter()
            .filter_map(|c| {
                let name = c.names.as_ref()?.first()?.trim_start_matches('/').to_string();
                let (compose_project, compose_service) = extract_compose_info(&c.labels, &name);
                if !filter_names.is_empty() && !show_all {
                    if !matches_filter(&name, compose_project.as_deref(), &filter_names) {
                        return None;
                    }
                }
                Some(self.summary_to_info(
                    c,
                    &name,
                    compose_project,
                    compose_service,
                    stats.get(&name),
                ))
            })
            .collect();

        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }

    fn summary_to_info(
        &self,
        c: ContainerSummary,
        name: &str,
        compose_project: Option<String>,
        compose_service: Option<String>,
        stats: Option<&ContainerStats>,
    ) -> ContainerInfo {
        let state = c.state.clone().unwrap_or_else(|| "unknown".into());
        let status = c.status.clone().unwrap_or_else(|| "unknown".into());

        let health = c.status.as_ref().and_then(|s| {
            if s.contains("(healthy)") {
                Some("healthy".into())
            } else if s.contains("(unhealthy)") {
                Some("unhealthy".into())
            } else if s.contains("(health: starting)") {
                Some("starting".into())
            } else {
                None
            }
        });

        let (cpu, mem_usage, mem_limit) = if state.eq_ignore_ascii_case("running") {
            stats
                .map(|s| (s.cpu_percent, s.memory_usage, s.memory_limit))
                .unwrap_or((0.0, 0, 0))
        } else {
            (0.0, 0, 0)
        };

        let mem_percent = if mem_limit > 0 {
            (mem_usage as f64 / mem_limit as f64) * 100.0
        } else {
            0.0
        };

        ContainerInfo {
            id: c.id.unwrap_or_default(),
            name: name.to_string(),
            image: c
                .image
                .unwrap_or_default()
                .split(':')
                .next()
                .unwrap_or("")
                .to_string(),
            status,
            state,
            health,
            cpu_percent: cpu,
            memory_usage: mem_usage,
            memory_limit: mem_limit,
            memory_percent: mem_percent,
            compose_project,
            compose_service,
        }
    }

    pub async fn start_container(&self, id: &str) -> AppResult<()> {
        self.docker()?
            .start_container(id, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| AppError::Docker(e.to_string()))?;
        Ok(())
    }

    pub async fn stop_container(&self, id: &str) -> AppResult<()> {
        self.docker()?
            .stop_container(id, None::<StopContainerOptions>)
            .await
            .map_err(|e| AppError::Docker(e.to_string()))?;
        Ok(())
    }

    pub async fn restart_container(&self, id: &str) -> AppResult<()> {
        self.docker()?
            .restart_container(id, None::<RestartContainerOptions>)
            .await
            .map_err(|e| AppError::Docker(e.to_string()))?;
        Ok(())
    }

    pub fn spawn_updates_checker(self: Arc<Self>, config: Arc<ConfigManager>) {
        if self.docker.is_none() {
            return;
        }
        tokio::spawn(async move {
            loop {
                let cfg = config.get().await;
                let refresh_hours = cfg.docker.refresh_hours.max(1);

                for (filter, show_all) in collect_updates_queries(&cfg) {
                    if let Err(e) = self
                        .list_updates(&filter, show_all, true, refresh_hours)
                        .await
                    {
                        tracing::warn!(
                            "Background docker updates check failed (show_all={show_all}): {e}"
                        );
                    }
                }

                let ttl_secs = refresh_hours.saturating_mul(3600).max(60);
                tokio::time::sleep(Duration::from_secs(ttl_secs)).await;
            }
        });
    }

    pub fn spawn_stats_collector(self: Arc<Self>) {
        if self.docker.is_none() {
            return;
        }
        tokio::spawn(async move {
            loop {
                if let Err(e) = self.collect_stats().await {
                    tracing::debug!("Stats collection error: {e}");
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        });
    }

    async fn collect_stats(&self) -> anyhow::Result<()> {
        let docker = match self.docker.as_ref() {
            Some(d) => d,
            None => return Ok(()),
        };

        let containers = docker
            .list_containers(None::<ListContainersOptions<String>>)
            .await?;

        let mut running_names = std::collections::HashSet::new();

        for container in containers {
            let id = match container.id {
                Some(id) => id,
                None => continue,
            };
            let name = container
                .names
                .as_ref()
                .and_then(|n| n.first())
                .map(|n| n.trim_start_matches('/').to_string())
                .unwrap_or_default();

            let state = container.state.as_deref().unwrap_or("");
            if state != "running" {
                continue;
            }

            running_names.insert(name.clone());

            let mut stream = docker.stats(
                &id,
                Some(StatsOptions {
                    stream: false,
                    one_shot: true,
                }),
            );

            if let Some(result) = stream.next().await {
                if let Ok(stats) = result {
                    let previous = self.stats_cache.read().await.get(&name).cloned();
                    let parsed = parse_stats(&stats, previous.as_ref());
                    self.stats_cache.write().await.insert(name, parsed);
                }
            }
        }

        self.stats_cache
            .write()
            .await
            .retain(|name, _| running_names.contains(name));

        Ok(())
    }
}

fn resolve_docker_socket(configured: &str) -> String {
    if Path::new(configured).exists() {
        return configured.to_string();
    }

    if let Ok(home) = std::env::var("HOME") {
        let mac = format!("{home}/.docker/run/docker.sock");
        if Path::new(&mac).exists() {
            tracing::info!("Using Docker socket at {mac}");
            return mac;
        }
    }

    configured.to_string()
}

fn parse_stats(stats: &Stats, previous: Option<&ContainerStats>) -> ContainerStats {
    let cpu_percent = calc_cpu_percent(stats, previous);
    let (memory_usage, memory_limit) = calc_memory(&stats.memory_stats);
    ContainerStats {
        cpu_percent,
        memory_usage,
        memory_limit,
        cpu_total_usage: stats.cpu_stats.cpu_usage.total_usage,
        system_cpu_usage: stats.cpu_stats.system_cpu_usage.unwrap_or(0),
    }
}

pub fn matches_filter(name: &str, compose_project: Option<&str>, filters: &[String]) -> bool {
    filters.iter().any(|f| {
        name.contains(f)
            || compose_project
                .map(|p| p == f.as_str() || p.contains(f.as_str()))
                .unwrap_or(false)
    })
}

pub fn extract_compose_info(
    labels: &Option<HashMap<String, String>>,
    name: &str,
) -> (Option<String>, Option<String>) {
    let project = labels
        .as_ref()
        .and_then(|l| l.get("com.docker.compose.project").cloned());
    let service = labels
        .as_ref()
        .and_then(|l| l.get("com.docker.compose.service").cloned());

    if project.is_some() {
        return (project, service);
    }

    parse_compose_name(name)
}

fn parse_compose_name(name: &str) -> (Option<String>, Option<String>) {
    let parts: Vec<&str> = name.split('-').collect();
    if parts.len() < 3 {
        return (None, None);
    }
    if parts.last().and_then(|s| s.parse::<u32>().ok()).is_none() {
        return (None, None);
    }
    let service = parts[parts.len() - 2].to_string();
    let project = parts[..parts.len() - 2].join("-");
    if project.is_empty() {
        return (None, None);
    }
    (Some(project), Some(service))
}

fn calc_cpu_percent(stats: &Stats, previous: Option<&ContainerStats>) -> f64 {
    let cpu_total = stats.cpu_stats.cpu_usage.total_usage;
    let system_cpu = stats.cpu_stats.system_cpu_usage.unwrap_or(0);

    let Some(prev) = previous else {
        return 0.0;
    };

    // Counters reset after container restart — wait for next sample.
    if cpu_total < prev.cpu_total_usage || system_cpu < prev.system_cpu_usage {
        return 0.0;
    }

    let cpu_delta = cpu_total - prev.cpu_total_usage;
    let system_delta = system_cpu - prev.system_cpu_usage;

    if system_delta > 0 && cpu_delta > 0 {
        let num_cpus = stats.cpu_stats.online_cpus.unwrap_or(1) as f64;
        (cpu_delta as f64 / system_delta as f64) * num_cpus * 100.0
    } else {
        0.0
    }
}

fn calc_memory(stats: &MemoryStats) -> (u64, u64) {
    let usage = stats.usage.unwrap_or(0);
    let limit = stats.limit.unwrap_or(0);
    (usage, limit)
}

fn collect_updates_queries(config: &AppConfig) -> Vec<(Vec<String>, bool)> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();

    for widget in &config.widgets {
        let WidgetConfig::DockerUpdates {
            containers,
            show_all,
            ..
        } = widget
        else {
            continue;
        };

        let key =
            crate::services::docker_updates::DockerUpdatesService::cache_key(containers, *show_all);
        if seen.insert(key) {
            out.push((containers.clone(), *show_all));
        }
    }

    out
}
