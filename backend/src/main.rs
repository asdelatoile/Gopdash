mod config;
mod error;
mod middleware;
mod routes;
mod services;
mod state;

use axum::{
    middleware as axum_middleware,
    Router,
};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::{
    compression::{CompressionLayer, predicate::NotForContentType},
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use notify::Watcher;

use crate::config::ConfigSource;
use crate::middleware::auth::auth_middleware;
use crate::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "gopdash=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config_source = ConfigSource::from_env();
    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "./static".into());
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    tracing::info!("Starting GopDash");
    tracing::info!("Config: {}", config_source.describe());
    tracing::info!("Static dir: {static_dir}");

    let state = Arc::new(AppState::new(config_source).await?);

    // Start background tasks
    state.docker.clone().spawn_stats_collector();
    state.docker.clone().spawn_updates_checker(state.config.clone());
    let watch_target = state.config.source().watch_target();
    spawn_config_watcher(state.clone(), watch_target);

    let public_api = routes::auth::router();

    let protected_api = Router::new()
        .merge(routes::api::router())
        .merge(routes::sse::router())
        .layer(axum_middleware::from_fn_with_state(state.clone(), auth_middleware));

    let api_routes = public_api.merge(protected_api);

    let static_path = static_dir.clone();
    let spa_fallback = ServeDir::new(&static_path)
        .not_found_service(ServeFile::new(format!("{static_path}/index.html")));

    let app = Router::new()
        .nest("/api", api_routes)
        .fallback_service(spa_fallback)
        .layer(
            CompressionLayer::new()
                .compress_when(NotForContentType::const_new("text/event-stream")),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Listening on http://{addr}");
    axum::serve(listener, app).await?;

    Ok(())
}

fn spawn_config_watcher(state: Arc<AppState>, path: PathBuf) {
    tokio::spawn(async move {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let mut watcher = match notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        }) {
            Ok(w) => w,
            Err(e) => {
                tracing::warn!("Config watcher disabled: {e}");
                return;
            }
        };

        if let Err(e) = watcher.watch(&path, notify::RecursiveMode::NonRecursive) {
            tracing::warn!("Failed to watch config file: {e}");
            return;
        }

        tracing::info!("Watching config file for changes: {}", path.display());

        while let Some(res) = rx.recv().await {
            match res {
                Ok(event) if is_config_change(&event) => {
                    // Debounce: editors often emit several modify events per save
                    loop {
                        tokio::select! {
                            _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)) => break,
                            Some(more) = rx.recv() => {
                                if let Ok(ev) = more {
                                    if !is_config_change(&ev) {
                                        continue;
                                    }
                                }
                            }
                        }
                    }

                    if let Err(e) = state.config.reload().await {
                        tracing::error!("Config reload failed: {e}");
                    } else {
                        let _ = state.config_tx.send(());
                    }
                }
                Ok(_) => {}
                Err(e) => tracing::error!("Config watcher error: {e}"),
            }
        }
    });
}

fn is_config_change(event: &notify::Event) -> bool {
    matches!(
        event.kind,
        notify::EventKind::Modify(_) | notify::EventKind::Create(_)
    )
}
