use crate::config::{ConfigManager, ConfigSource};
use crate::services::docker::DockerService;
use crate::services::health::HealthService;
use crate::services::jellyfin::JellyfinService;
use crate::services::rss::RssService;
use crate::services::session::SessionStore;
use crate::services::system::SystemService;
use crate::services::weather::WeatherService;
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<ConfigManager>,
    pub docker: Arc<DockerService>,
    pub system: Arc<SystemService>,
    pub weather: Arc<WeatherService>,
    pub rss: Arc<RssService>,
    pub health: Arc<HealthService>,
    pub jellyfin: Arc<JellyfinService>,
    pub sessions: Arc<SessionStore>,
    pub config_tx: broadcast::Sender<()>,
}

impl AppState {
    pub async fn new(source: ConfigSource) -> anyhow::Result<Self> {
        let config = Arc::new(ConfigManager::new(source)?);
        let cfg = config.get().await;

        let docker = Arc::new(DockerService::new(&cfg.docker.socket_path).await?);
        let system = Arc::new(SystemService::new());
        let weather = Arc::new(WeatherService::new());
        let rss = Arc::new(RssService::new());
        let health = Arc::new(HealthService::new());
        let jellyfin = Arc::new(JellyfinService::new());
        let sessions = Arc::new(SessionStore::new());
        let (config_tx, _) = broadcast::channel(16);

        Ok(Self {
            config,
            docker,
            system,
            weather,
            rss,
            health,
            jellyfin,
            sessions,
            config_tx,
        })
    }
}
