use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    #[serde(default = "default_title")]
    pub title: String,

    #[serde(default)]
    pub auth: AuthConfig,

    #[serde(default = "default_refresh_interval")]
    pub refresh_interval: u64,

    #[serde(default)]
    pub widgets: Vec<WidgetConfig>,

    #[serde(default)]
    pub weather: Option<WeatherConfig>,

    #[serde(default)]
    pub bookmarks: Vec<BookmarkGroup>,

    #[serde(default)]
    pub rss: Vec<RssFeedConfig>,

    #[serde(default)]
    pub docker: DockerConfig,

    #[serde(default)]
    pub search_engines: Vec<SearchEngineConfig>,

    #[serde(default)]
    pub jellyfin: Option<JellyfinConfig>,

    #[serde(default)]
    pub theme: ThemeConfig,

    #[serde(default)]
    pub settings: SettingsConfig,
}

fn default_title() -> String {
    "GopDash".into()
}

fn default_refresh_interval() -> u64 {
    5
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AuthConfig {
    #[serde(default)]
    pub enabled: bool,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum WidgetConfig {
    Docker {
        id: String,
        title: Option<String>,
        icon: Option<String>,
        #[serde(default)]
        x: i32,
        #[serde(default)]
        y: i32,
        #[serde(default = "default_w")]
        w: i32,
        #[serde(default = "default_h")]
        h: i32,
        #[serde(default)]
        containers: Vec<String>,
        #[serde(default)]
        show_all: bool,
        #[serde(default)]
        group_by: DockerGroupBy,
        #[serde(default = "default_collapse_groups")]
        collapse_groups: bool,
        #[serde(default = "default_show_header")]
        show_header: bool,
    },
    #[serde(rename = "docker-updates")]
    DockerUpdates {
        id: String,
        title: Option<String>,
        icon: Option<String>,
        #[serde(default)]
        x: i32,
        #[serde(default)]
        y: i32,
        #[serde(default = "default_w")]
        w: i32,
        #[serde(default = "default_h")]
        h: i32,
        #[serde(default)]
        containers: Vec<String>,
        #[serde(default)]
        show_all: bool,
        #[serde(default = "default_show_header")]
        show_header: bool,
    },
    #[serde(rename = "docker-stack")]
    DockerStack {
        id: String,
        title: Option<String>,
        icon: Option<String>,
        #[serde(default)]
        x: i32,
        #[serde(default)]
        y: i32,
        #[serde(default = "default_w")]
        w: i32,
        #[serde(default = "default_h")]
        h: i32,
        #[serde(default = "default_show_header")]
        show_header: bool,
        /// Groupes de containers / stacks Compose à contrôler ensemble
        #[serde(default)]
        stacks: Vec<DockerStackGroup>,
    },
    System {
        id: String,
        title: Option<String>,
        icon: Option<String>,
        #[serde(default)]
        x: i32,
        #[serde(default)]
        y: i32,
        #[serde(default = "default_w")]
        w: i32,
        #[serde(default = "default_h")]
        h: i32,
        #[serde(default = "default_show_header")]
        show_header: bool,
    },
    Weather {
        id: String,
        title: Option<String>,
        icon: Option<String>,
        #[serde(default)]
        x: i32,
        #[serde(default)]
        y: i32,
        #[serde(default = "default_w")]
        w: i32,
        #[serde(default = "default_h")]
        h: i32,
        location: Option<String>,
        units: Option<String>,
        /// Surcharge le réglage global `weather.show_forecast` de services.yaml
        show_forecast: Option<bool>,
        #[serde(default = "default_show_header")]
        show_header: bool,
    },
    Bookmarks {
        id: String,
        title: Option<String>,
        icon: Option<String>,
        #[serde(default)]
        x: i32,
        #[serde(default)]
        y: i32,
        #[serde(default = "default_w")]
        w: i32,
        #[serde(default = "default_h")]
        h: i32,
        /// ID de l'entrée dans services.yaml → bookmarks[].id
        service_id: String,
        #[serde(default = "default_bookmark_columns")]
        columns: u32,
        #[serde(default = "default_show_header")]
        show_header: bool,
    },
    Rss {
        id: String,
        title: Option<String>,
        icon: Option<String>,
        #[serde(default)]
        x: i32,
        #[serde(default)]
        y: i32,
        #[serde(default = "default_w")]
        w: i32,
        #[serde(default = "default_h")]
        h: i32,
        service_id: String,
        #[serde(default = "default_max_items")]
        max_items: u32,
        #[serde(default = "default_show_header")]
        show_header: bool,
    },
    Calendar {
        id: String,
        title: Option<String>,
        icon: Option<String>,
        #[serde(default)]
        x: i32,
        #[serde(default)]
        y: i32,
        #[serde(default = "default_w")]
        w: i32,
        #[serde(default = "default_h")]
        h: i32,
        #[serde(default = "default_show_header")]
        show_header: bool,
        /// Affiche le jour et l'heure actuels au-dessus de la grille
        #[serde(default = "default_show_today")]
        show_today: bool,
        /// Affiche les jours des mois précédent/suivant dans la grille
        #[serde(default)]
        show_outside_days: bool,
        /// Affiche les flèches pour changer de mois
        #[serde(default)]
        show_navigation: bool,
    },
    Search {
        id: String,
        title: Option<String>,
        icon: Option<String>,
        #[serde(default)]
        x: i32,
        #[serde(default)]
        y: i32,
        #[serde(default = "default_w")]
        w: i32,
        #[serde(default = "default_h")]
        h: i32,
        #[serde(default = "default_show_header")]
        show_header: bool,
        /// ID de l'entrée dans services.yaml → search_engines[].id
        service_id: String,
        #[serde(default)]
        target: SearchTarget,
    },
    Jellyfin {
        id: String,
        title: Option<String>,
        icon: Option<String>,
        #[serde(default)]
        x: i32,
        #[serde(default)]
        y: i32,
        #[serde(default = "default_w")]
        w: i32,
        #[serde(default = "default_h")]
        h: i32,
        #[serde(default = "default_show_header")]
        show_header: bool,
        #[serde(default = "default_show_now_playing")]
        show_now_playing: bool,
        #[serde(default = "default_show_library_counts")]
        show_library_counts: bool,
        #[serde(default = "default_max_sessions")]
        max_sessions: u32,
    },
}

fn default_w() -> i32 {
    4
}
fn default_h() -> i32 {
    3
}
fn default_collapse_groups() -> bool {
    true
}

fn default_bookmark_columns() -> u32 {
    3
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DockerGroupBy {
    #[default]
    Flat,
    Compose,
}
fn default_show_forecast() -> bool {
    true
}

fn default_show_now_playing() -> bool {
    true
}

fn default_show_library_counts() -> bool {
    true
}

fn default_max_sessions() -> u32 {
    3
}

fn default_show_header() -> bool {
    true
}

fn default_show_today() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SearchTarget {
    #[default]
    NewTab,
    SameTab,
}

fn default_max_items() -> u32 {
    10
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThemeConfig {
    #[serde(default = "default_theme_mode")]
    pub mode: ThemeMode,
    #[serde(default = "default_theme_preset")]
    pub preset: ThemePreset,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            mode: default_theme_mode(),
            preset: default_theme_preset(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Light,
    Dark,
    System,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreset {
    Default,
    Ocean,
    Forest,
    Rose,
}

fn default_theme_mode() -> ThemeMode {
    ThemeMode::System
}

fn default_theme_preset() -> ThemePreset {
    ThemePreset::Default
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GridSettings {
    #[serde(default = "default_grid_columns")]
    pub columns: u32,
    #[serde(default = "default_grid_cell_height")]
    pub cell_height: u32,
}

impl Default for GridSettings {
    fn default() -> Self {
        Self {
            columns: default_grid_columns(),
            cell_height: default_grid_cell_height(),
        }
    }
}

fn default_grid_columns() -> u32 {
    24
}

fn default_grid_cell_height() -> u32 {
    15
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SettingsConfig {
    #[serde(default = "default_persist_layout")]
    pub persist_layout: bool,
    /// BCP 47, ex. fr-FR, en-US
    #[serde(default = "default_locale")]
    pub locale: String,
    /// IANA, ex. Europe/Paris
    #[serde(default = "default_timezone")]
    pub timezone: String,
    #[serde(default)]
    pub grid: GridSettings,
}

impl Default for SettingsConfig {
    fn default() -> Self {
        Self {
            persist_layout: default_persist_layout(),
            locale: default_locale(),
            timezone: default_timezone(),
            grid: GridSettings::default(),
        }
    }
}

fn default_persist_layout() -> bool {
    true
}

fn default_locale() -> String {
    "fr-FR".into()
}

fn default_timezone() -> String {
    "Europe/Paris".into()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WidgetLayoutUpdate {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl WidgetConfig {
    pub fn id(&self) -> &str {
        match self {
            Self::Docker { id, .. }
            | Self::DockerUpdates { id, .. }
            | Self::DockerStack { id, .. }
            | Self::System { id, .. }
            | Self::Weather { id, .. }
            | Self::Bookmarks { id, .. }
            | Self::Rss { id, .. }
            | Self::Calendar { id, .. }
            | Self::Search { id, .. }
            | Self::Jellyfin { id, .. } => id,
        }
    }

    pub fn set_layout(&mut self, x: i32, y: i32, w: i32, h: i32) {
        match self {
            Self::Docker { x: px, y: py, w: pw, h: ph, .. }
            | Self::DockerUpdates { x: px, y: py, w: pw, h: ph, .. }
            | Self::DockerStack { x: px, y: py, w: pw, h: ph, .. }
            | Self::System { x: px, y: py, w: pw, h: ph, .. }
            | Self::Weather { x: px, y: py, w: pw, h: ph, .. }
            | Self::Bookmarks { x: px, y: py, w: pw, h: ph, .. }
            | Self::Rss { x: px, y: py, w: pw, h: ph, .. }
            | Self::Calendar { x: px, y: py, w: pw, h: ph, .. }
            | Self::Search { x: px, y: py, w: pw, h: ph, .. }
            | Self::Jellyfin { x: px, y: py, w: pw, h: ph, .. } => {
                *px = x;
                *py = y;
                *pw = w;
                *ph = h;
            }
        }
    }

    pub fn layout(&self) -> (i32, i32, i32, i32) {
        match self {
            Self::Docker { x, y, w, h, .. }
            | Self::DockerUpdates { x, y, w, h, .. }
            | Self::DockerStack { x, y, w, h, .. }
            | Self::System { x, y, w, h, .. }
            | Self::Weather { x, y, w, h, .. }
            | Self::Bookmarks { x, y, w, h, .. }
            | Self::Rss { x, y, w, h, .. }
            | Self::Calendar { x, y, w, h, .. }
            | Self::Search { x, y, w, h, .. }
            | Self::Jellyfin { x, y, w, h, .. } => (*x, *y, *w, *h),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JellyfinConfig {
    pub url: String,
    pub api_key: String,
    #[serde(default)]
    pub insecure: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WeatherConfig {
    pub default_location: String,
    #[serde(default = "default_units")]
    pub units: String,
    #[serde(default = "default_show_forecast")]
    pub show_forecast: bool,
}

fn default_units() -> String {
    "metric".into()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DockerStackGroup {
    pub name: String,
    /// Noms de containers ou projets Compose (filtre partiel, comme le widget docker)
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchEngineConfig {
    pub id: String,
    pub name: String,
    /// URL avec placeholder `{query}` (ex. https://google.com/search?q={query})
    pub url: String,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BookmarkGroup {
    pub id: String,
    /// Libellé optionnel (affichage / debug)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub links: Vec<BookmarkLink>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BookmarkLink {
    pub name: String,
    pub url: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    /// Active le ping HTTP (pastille up/down sur le widget bookmarks)
    #[serde(default)]
    pub health_check: bool,
    /// URL à sonder (défaut : url du lien)
    pub health_url: Option<String>,
    #[serde(default = "default_health_method")]
    pub method: String,
    #[serde(default = "default_health_timeout")]
    pub timeout_secs: u64,
    /// Code HTTP attendu (sinon 2xx = OK)
    pub expected_status: Option<u16>,
    /// Accepter les certificats TLS auto-signés
    #[serde(default)]
    pub insecure: bool,
}

fn default_health_method() -> String {
    "GET".into()
}

fn default_health_timeout() -> u64 {
    5
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RssFeedConfig {
    pub id: String,
    pub url: String,
    #[serde(default = "default_rss_refresh")]
    pub refresh_minutes: u64,
}

fn default_rss_refresh() -> u64 {
    30
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DockerConfig {
    #[serde(default = "default_docker_socket")]
    pub socket_path: String,
    /// Intervalle entre deux vérifications des mises à jour d'images (widget docker-updates).
    #[serde(default = "default_docker_refresh_hours")]
    pub refresh_hours: u64,
}

impl Default for DockerConfig {
    fn default() -> Self {
        Self {
            socket_path: default_docker_socket(),
            refresh_hours: default_docker_refresh_hours(),
        }
    }
}

fn default_docker_socket() -> String {
    "/var/run/docker.sock".into()
}

fn default_docker_refresh_hours() -> u64 {
    4
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct WidgetLayout {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// `config/app.yaml` — réglages globaux édités à la main.
#[derive(Debug, Deserialize, Serialize)]
struct AppFile {
    #[serde(default = "default_title")]
    title: String,
    #[serde(default)]
    auth: AuthConfig,
    #[serde(default = "default_refresh_interval")]
    refresh_interval: u64,
    #[serde(default)]
    theme: ThemeConfig,
    #[serde(default)]
    settings: SettingsConfig,
}

/// `config/dashboard.yaml` — définitions des widgets (sans positions).
#[derive(Debug, Deserialize, Serialize)]
struct DashboardFile {
    #[serde(default)]
    widgets: Vec<WidgetConfig>,
}

/// `config/services.yaml` — sources de données.
#[derive(Debug, Deserialize, Serialize)]
struct ServicesFile {
    #[serde(default)]
    weather: Option<WeatherConfig>,
    #[serde(default)]
    bookmarks: Vec<BookmarkGroup>,
    #[serde(default)]
    rss: Vec<RssFeedConfig>,
    #[serde(default)]
    docker: DockerConfig,
    #[serde(default)]
    search_engines: Vec<SearchEngineConfig>,
    #[serde(default)]
    jellyfin: Option<JellyfinConfig>,
}

/// `config/layout.yaml` — positions Gridstack, écrit par le backend.
#[derive(Debug, Default, Deserialize, Serialize)]
struct LayoutFile {
    #[serde(default)]
    layouts: BTreeMap<String, WidgetLayout>,
}

fn read_file_or_empty<T: DeserializeOwned>(path: &Path) -> anyhow::Result<T> {
    let content = if path.exists() {
        std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read {}: {e}", path.display()))?
    } else {
        "{}".to_string()
    };
    serde_yaml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse {}: {e}", path.display()))
}

fn merge_files(
    app: AppFile,
    dashboard: DashboardFile,
    services: ServicesFile,
    layout: LayoutFile,
) -> AppConfig {
    let mut widgets = dashboard.widgets;
    for widget in widgets.iter_mut() {
        if let Some(pos) = layout.layouts.get(widget.id()) {
            widget.set_layout(pos.x, pos.y, pos.w, pos.h);
        }
    }

    AppConfig {
        title: app.title,
        auth: app.auth,
        refresh_interval: app.refresh_interval,
        widgets,
        weather: services.weather,
        bookmarks: services.bookmarks,
        rss: services.rss,
        docker: services.docker,
        search_engines: services.search_engines,
        jellyfin: services.jellyfin,
        theme: app.theme,
        settings: app.settings,
    }
}

/// Où lire la configuration : un dossier éclaté (recommandé) ou un fichier unique (legacy).
#[derive(Debug, Clone)]
pub enum ConfigSource {
    Dir(PathBuf),
    File(PathBuf),
}

impl ConfigSource {
    /// Résout la source via l'environnement :
    /// - `CONFIG_DIR` → dossier explicite
    /// - sinon `./config/` s'il existe
    /// - sinon `CONFIG_PATH` (fichier unique, legacy)
    /// - sinon `./config/` par défaut (créé au besoin)
    pub fn from_env() -> Self {
        if let Ok(dir) = std::env::var("CONFIG_DIR") {
            return Self::Dir(PathBuf::from(dir));
        }

        let default_dir = PathBuf::from("config");
        if default_dir.is_dir() {
            return Self::Dir(default_dir);
        }

        if let Ok(path) = std::env::var("CONFIG_PATH") {
            let legacy = PathBuf::from(path);
            if legacy.exists() {
                return Self::File(legacy);
            }
        }

        Self::Dir(default_dir)
    }

    pub fn watch_target(&self) -> PathBuf {
        match self {
            Self::Dir(dir) => dir.clone(),
            Self::File(path) => path.clone(),
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Self::Dir(dir) => format!("directory {}", dir.display()),
            Self::File(path) => format!("file {}", path.display()),
        }
    }
}

pub struct ConfigManager {
    source: ConfigSource,
    config: Arc<RwLock<AppConfig>>,
}

impl ConfigManager {
    pub fn new(source: ConfigSource) -> anyhow::Result<Self> {
        let config = Self::load(&source)?;
        Ok(Self {
            source,
            config: Arc::new(RwLock::new(config)),
        })
    }

    pub fn source(&self) -> &ConfigSource {
        &self.source
    }

    fn load(source: &ConfigSource) -> anyhow::Result<AppConfig> {
        match source {
            ConfigSource::Dir(dir) => Self::load_from_dir(dir),
            ConfigSource::File(path) => Self::load_from_file(path),
        }
    }

    fn load_from_dir(dir: &Path) -> anyhow::Result<AppConfig> {
        let app: AppFile = read_file_or_empty(&dir.join("app.yaml"))?;
        let dashboard: DashboardFile = read_file_or_empty(&dir.join("dashboard.yaml"))?;
        let services: ServicesFile = read_file_or_empty(&dir.join("services.yaml"))?;
        let layout: LayoutFile = read_file_or_empty(&dir.join("layout.yaml"))?;
        Ok(merge_files(app, dashboard, services, layout))
    }

    fn load_from_file(path: &Path) -> anyhow::Result<AppConfig> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read config at {}: {e}", path.display()))?;
        let config: AppConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    pub async fn get(&self) -> AppConfig {
        self.config.read().await.clone()
    }

    pub async fn reload(&self) -> anyhow::Result<()> {
        let source = self.source.clone();
        let new_config = tokio::task::spawn_blocking(move || Self::load(&source))
            .await
            .map_err(|e| anyhow::anyhow!("Config reload task failed: {e}"))??;
        *self.config.write().await = new_config;
        tracing::info!("Configuration reloaded from {}", self.source.describe());
        Ok(())
    }

    pub async fn update_layouts(&self, layouts: &[WidgetLayoutUpdate]) -> anyhow::Result<()> {
        {
            let mut config = self.config.write().await;
            for layout in layouts {
                if let Some(widget) = config.widgets.iter_mut().find(|w| w.id() == layout.id) {
                    widget.set_layout(layout.x, layout.y, layout.w, layout.h);
                }
            }
        }

        match self.source.clone() {
            ConfigSource::Dir(dir) => self.write_layout_file(&dir).await,
            ConfigSource::File(path) => self.write_full_file(&path).await,
        }
    }

    /// Mode dossier : n'écrit que `layout.yaml`, sans toucher aux fichiers édités à la main.
    async fn write_layout_file(&self, dir: &Path) -> anyhow::Result<()> {
        let file = {
            let config = self.config.read().await;
            let layouts = config
                .widgets
                .iter()
                .map(|w| {
                    let (x, y, width, h) = w.layout();
                    (
                        w.id().to_string(),
                        WidgetLayout { x, y, w: width, h },
                    )
                })
                .collect();
            LayoutFile { layouts }
        };

        let yaml = tokio::task::spawn_blocking(move || serde_yaml::to_string(&file))
            .await
            .map_err(|e| anyhow::anyhow!("Layout serialize task failed: {e}"))??;

        tokio::fs::create_dir_all(dir).await.ok();
        let path = dir.join("layout.yaml");
        tokio::fs::write(&path, yaml).await?;
        tracing::info!("Layout saved to {}", path.display());
        Ok(())
    }

    /// Mode legacy fichier unique : ré-sérialise tout le fichier.
    async fn write_full_file(&self, path: &Path) -> anyhow::Result<()> {
        let snapshot = self.config.read().await.clone();
        let yaml = tokio::task::spawn_blocking(move || serde_yaml::to_string(&snapshot))
            .await
            .map_err(|e| anyhow::anyhow!("Layout serialize task failed: {e}"))??;

        tokio::fs::write(path, yaml).await?;
        tracing::info!("Layout saved to {}", path.display());
        Ok(())
    }
}
