use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
    routing::get,
    Router,
};
use futures::stream::Stream;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/events", get(sse_handler))
}

async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let config = state.config.get().await;
    let interval_secs = config.refresh_interval.max(5);

    let system = state.system.clone();
    let docker = state.docker.clone();

    // First event sent immediately, then every `interval_secs`
    let stream = futures::stream::unfold(true, move |first| {
        let system = system.clone();
        let docker = docker.clone();
        async move {
            if !first {
                tokio::time::sleep(Duration::from_secs(interval_secs)).await;
            }

            let metrics = system.collect().await;
            let containers = docker.list_containers(&[], true).await.unwrap_or_default();

            let payload = serde_json::json!({
                "type": "update",
                "system": metrics,
                "docker": containers,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });

            let event = Event::default().event("update").data(payload.to_string());
            Some((Ok(event), false))
        }
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}
