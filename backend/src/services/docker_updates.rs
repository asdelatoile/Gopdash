use crate::error::{AppError, AppResult};
use crate::services::docker::{extract_compose_info, matches_filter};
use bollard::container::{
    Config, CreateContainerOptions, InspectContainerOptions, ListContainersOptions,
    NetworkingConfig, RemoveContainerOptions, StopContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::models::ContainerSummary;
use bollard::Docker;
use futures::StreamExt;
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use reqwest::Client;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpdateStatus {
    UpToDate,
    Available,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContainerUpdateInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: String,
    pub compose_project: Option<String>,
    pub compose_service: Option<String>,
    pub current_digest: Option<String>,
    pub remote_digest: Option<String>,
    pub status: UpdateStatus,
    pub error: Option<String>,
}

struct CachedRemoteDigest {
    digest: Option<String>,
    fetched_at: Instant,
}

pub struct DockerUpdatesService {
    client: Client,
    cache: Arc<RwLock<HashMap<String, CachedRemoteDigest>>>,
}

impl DockerUpdatesService {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Gopdash/0.1 (docker-updates)")
                .timeout(Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn list_updates(
        &self,
        docker: &Docker,
        filter_names: &[String],
        show_all: bool,
    ) -> AppResult<Vec<ContainerUpdateInfo>> {
        let options = Some(ListContainersOptions::<String> {
            all: show_all,
            ..Default::default()
        });

        let containers = docker
            .list_containers(options)
            .await
            .map_err(|e| AppError::Docker(e.to_string()))?;

        let mut result = Vec::new();
        for summary in containers {
            let Some(info) = self
                .inspect_update_info(docker, summary, filter_names, show_all)
                .await?
            else {
                continue;
            };
            result.push(info);
        }

        result.sort_by(|a, b| a.name.cmp(&b.name));
        result.retain(|item| item.status == UpdateStatus::Available);
        Ok(result)
    }

    pub async fn update_container(&self, docker: &Docker, id: &str) -> AppResult<()> {
        let inspect = docker
            .inspect_container(id, None::<InspectContainerOptions>)
            .await
            .map_err(|e| AppError::Docker(e.to_string()))?;

        let name = inspect
            .name
            .as_deref()
            .map(|n| n.trim_start_matches('/').to_string())
            .ok_or_else(|| AppError::Docker("Container name missing".into()))?;

        let image = inspect
            .config
            .as_ref()
            .and_then(|c| c.image.clone())
            .ok_or_else(|| AppError::Docker("Container image missing".into()))?;

        pull_image(docker, &image).await?;

        let was_running = inspect
            .state
            .as_ref()
            .and_then(|s| s.running)
            .unwrap_or(false);

        if was_running {
            docker
                .stop_container(id, None::<StopContainerOptions>)
                .await
                .map_err(|e| AppError::Docker(e.to_string()))?;
        }

        docker
            .remove_container(
                id,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await
            .map_err(|e| AppError::Docker(e.to_string()))?;

        let create_config = build_create_config(&inspect, &image)?;
        let create_options = CreateContainerOptions {
            name: name.clone(),
            platform: inspect.platform.clone(),
        };

        let created = docker
            .create_container(Some(create_options), create_config)
            .await
            .map_err(|e| AppError::Docker(e.to_string()))?;

        if was_running {
            docker
                .start_container(&created.id, None::<bollard::container::StartContainerOptions<String>>)
                .await
                .map_err(|e| AppError::Docker(e.to_string()))?;
        }

        self.cache.write().await.remove(&image);
        Ok(())
    }

    async fn inspect_update_info(
        &self,
        docker: &Docker,
        summary: ContainerSummary,
        filter_names: &[String],
        show_all: bool,
    ) -> AppResult<Option<ContainerUpdateInfo>> {
        let id = summary.id.unwrap_or_default();
        let name = summary
            .names
            .as_ref()
            .and_then(|n| n.first())
            .map(|n| n.trim_start_matches('/').to_string())
            .unwrap_or_default();

        let (compose_project, compose_service) =
            extract_compose_info(&summary.labels, &name);

        if !filter_names.is_empty() && !show_all {
            if !matches_filter(
                &name,
                compose_project.as_deref(),
                filter_names,
            ) {
                return Ok(None);
            }
        }

        let inspect = docker
            .inspect_container(&id, None::<InspectContainerOptions>)
            .await
            .map_err(|e| AppError::Docker(e.to_string()))?;

        let image = inspect
            .config
            .as_ref()
            .and_then(|c| c.image.clone())
            .unwrap_or_else(|| summary.image.unwrap_or_default());

        let state = summary
            .state
            .unwrap_or_else(|| "unknown".into());

        if image.contains('@') {
            return Ok(Some(ContainerUpdateInfo {
                id,
                name,
                image,
                state,
                compose_project,
                compose_service,
                current_digest: None,
                remote_digest: None,
                status: UpdateStatus::Unknown,
                error: Some("Image pinned by digest".into()),
            }));
        }

        let current_digest = local_digest(docker, &image).await;
        let (remote_digest, error) = self.remote_digest(&image).await;
        let status = match (&current_digest, &remote_digest) {
            (Some(local), Some(remote)) if local == remote => UpdateStatus::UpToDate,
            (Some(_), Some(_)) => UpdateStatus::Available,
            _ => UpdateStatus::Unknown,
        };

        Ok(Some(ContainerUpdateInfo {
            id,
            name,
            image,
            state,
            compose_project,
            compose_service,
            current_digest,
            remote_digest,
            status,
            error,
        }))
    }

    async fn remote_digest(&self, image: &str) -> (Option<String>, Option<String>) {
        let Some(parsed) = parse_image_reference(image) else {
            return (None, Some("Unsupported image reference".into()));
        };

        if let Some(cached) = self.cache.read().await.get(image) {
            if cached.fetched_at.elapsed() < Duration::from_secs(300) {
                return (cached.digest.clone(), None);
            }
        }

        let result = fetch_remote_digest(&self.client, &parsed).await;
        let (digest, error) = match result {
            Ok(d) => (d, None),
            Err(e) => (None, Some(e)),
        };

        self.cache.write().await.insert(
            image.to_string(),
            CachedRemoteDigest {
                digest: digest.clone(),
                fetched_at: Instant::now(),
            },
        );

        (digest, error)
    }
}

impl Default for DockerUpdatesService {
    fn default() -> Self {
        Self::new()
    }
}

struct ParsedImage {
    registry: String,
    repository: String,
    tag: String,
}

fn parse_image_reference(image: &str) -> Option<ParsedImage> {
    let image = image.split('@').next()?.trim();
    if image.is_empty() {
        return None;
    }

    let mut registry = "registry-1.docker.io".to_string();
    let mut remainder = image;

    if let Some(idx) = image.find('/') {
        let head = &image[..idx];
        if head.contains('.') || head.contains(':') || head == "localhost" {
            registry = match head {
                "docker.io" => "registry-1.docker.io".into(),
                other => other.into(),
            };
            remainder = &image[idx + 1..];
        }
    }

    let (repository, tag) = match remainder.rsplit_once(':') {
        Some((repo, tag)) if !tag.contains('/') => (repo.to_string(), tag.to_string()),
        _ => (remainder.to_string(), "latest".to_string()),
    };

    let repository = if registry == "registry-1.docker.io" && !repository.contains('/') {
        format!("library/{repository}")
    } else {
        repository
    };

    Some(ParsedImage {
        registry,
        repository,
        tag,
    })
}

async fn local_digest(docker: &Docker, image: &str) -> Option<String> {
    let inspected = docker.inspect_image(image).await.ok()?;
    inspected
        .repo_digests
        .as_ref()
        .and_then(|digests| digests.first())
        .map(|d| normalize_digest(d))
        .or_else(|| inspected.id.as_ref().map(|d| normalize_digest(d)))
}

async fn fetch_remote_digest(client: &Client, parsed: &ParsedImage) -> Result<Option<String>, String> {
    let token = fetch_registry_token(client, parsed).await.ok();
    let manifest_url = format!(
        "https://{}/v2/{}/manifests/{}",
        parsed.registry, parsed.repository, parsed.tag
    );

    let mut request = client
        .get(&manifest_url)
        .header(
            ACCEPT,
            "application/vnd.docker.distribution.manifest.v2+json, application/vnd.oci.image.manifest.v1+json",
        );

    if let Some(token) = token {
        request = request.header(AUTHORIZATION, format!("Bearer {token}"));
    }

    let response = request
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("Registry HTTP {}", response.status()));
    }

    let digest = response
        .headers()
        .get("docker-content-digest")
        .and_then(|v| v.to_str().ok())
        .map(|d| normalize_digest(d));

    Ok(digest)
}

async fn fetch_registry_token(client: &Client, parsed: &ParsedImage) -> Result<String, String> {
    if parsed.registry == "registry-1.docker.io" {
        let url = format!(
            "https://auth.docker.io/token?service=registry.docker.io&scope=repository:{}:pull",
            parsed.repository
        );
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        return body
            .get("token")
            .and_then(|t| t.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| "Missing registry token".into());
    }

    if parsed.registry == "ghcr.io" {
        let url = format!(
            "https://ghcr.io/token?scope=repository:{}:pull",
            parsed.repository
        );
        let response = client
            .get(url)
            .header(USER_AGENT, "Gopdash/0.1")
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if response.status().is_success() {
            let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
            if let Some(token) = body.get("token").and_then(|t| t.as_str()) {
                return Ok(token.to_string());
            }
        }
    }

    Err("No registry token".into())
}

fn normalize_digest(digest: &str) -> String {
    let digest = digest.trim().to_ascii_lowercase();
    if let Some((_, hash)) = digest.split_once("@sha256:") {
        return hash.to_string();
    }
    digest.trim_start_matches("sha256:").to_string()
}

#[cfg(test)]
mod tests {
    use super::normalize_digest;

    #[test]
    fn normalize_repo_digest() {
        assert_eq!(
            normalize_digest("axllent/mailpit@sha256:ABC123"),
            "abc123"
        );
    }

    #[test]
    fn normalize_image_id() {
        assert_eq!(normalize_digest("sha256:def456"), "def456");
    }
}

async fn pull_image(docker: &Docker, image: &str) -> AppResult<()> {
    let (from_image, tag) = match image.rsplit_once(':') {
        Some((name, tag)) if !tag.contains('/') => (name, tag),
        _ => (image, "latest"),
    };

    let options = Some(CreateImageOptions {
        from_image,
        tag,
        ..Default::default()
    });

    let mut stream = docker.create_image(options, None, None);
    while let Some(result) = stream.next().await {
        result.map_err(|e| AppError::Docker(e.to_string()))?;
    }

    Ok(())
}

fn build_create_config(
    inspect: &bollard::models::ContainerInspectResponse,
    image: &str,
) -> AppResult<Config<String>> {
    let container_config = inspect
        .config
        .as_ref()
        .ok_or_else(|| AppError::Docker("Container config missing".into()))?;

    let json = serde_json::to_value(container_config)
        .map_err(|e| AppError::Internal(e.into()))?;
    let mut create_config: Config<String> = serde_json::from_value(json)
        .map_err(|e| AppError::Internal(e.into()))?;

    create_config.image = Some(image.to_string());
    create_config.host_config = inspect.host_config.clone();

    if let Some(networks) = inspect
        .network_settings
        .as_ref()
        .and_then(|ns| ns.networks.clone())
    {
        if !networks.is_empty() {
            create_config.networking_config = Some(NetworkingConfig {
                endpoints_config: networks,
            });
        }
    }

    Ok(create_config)
}
