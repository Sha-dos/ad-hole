use crate::analytics::Analytics;
use crate::blocklist::{Blocklist, Source};
use axum::extract::{Path, State};
use axum::http::{StatusCode, Uri, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use rust_embed::RustEmbed;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{error, info};

#[derive(RustEmbed)]
#[folder = "frontend/dist/"]
struct Assets;

#[derive(Clone)]
struct AppState {
    blocklist: Arc<Mutex<Blocklist>>,
    analytics: Arc<Analytics>,
}

pub struct Server;

#[derive(Deserialize)]
struct PatchUpdateFreq {
    update_freq: u64,
}

#[derive(Deserialize)]
struct PatchBlocklist {
    domain: String,
    action: String, // "add" or "remove"
}

#[derive(Deserialize)]
struct PatchSources {
    url: String,
    action: String, // "add", "remove", "toggle"
    enabled: Option<bool>,
}

impl Server {
    pub async fn run(blocklist: Arc<Mutex<Blocklist>>, analytics: Arc<Analytics>) {
        let state = AppState { blocklist, analytics };
        let app = Router::new()
            .route("/check_blocklist/{domain}", get(Self::check_blocklist))
            .route("/set_update_freq", post(Self::handle_update_freq))
            .route("/update_blocklist", post(Self::handle_update_blocklist))
            .route("/overrides", get(Self::handle_get_overrides))
            .route(
                "/sources",
                get(Self::handle_get_sources).post(Self::handle_patch_sources),
            )
            .route("/analytics", get(Self::handle_analytics_summary))
            .route("/analytics/top_blocked", get(Self::handle_top_blocked))
            .route("/analytics/top_queried", get(Self::handle_top_queried))
            .with_state(state)
            .fallback(Self::frontend);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
            .await
            .unwrap();
        axum::serve(listener, app).await.unwrap();
    }

    async fn frontend(uri: Uri) -> Response {
        let path = uri.path().trim_start_matches('/');

        match Assets::get(path) {
            Some(content) => {
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                (
                    [(header::CONTENT_TYPE, mime.as_ref().to_owned())],
                    content.data,
                )
                    .into_response()
            }
            None => match Assets::get("index.html") {
                Some(index) => Html(index.data).into_response(),
                None => StatusCode::NOT_FOUND.into_response(),
            },
        }
    }

    async fn check_blocklist(
        Path(domain): Path<String>,
        State(AppState { blocklist, .. }): State<AppState>,
    ) -> Json<Value> {
        let guard = blocklist.lock().await;

        Json(json!({
            "blocked": guard.check(&*domain),
        }))
    }

    async fn handle_update_freq(
        State(AppState { blocklist, .. }): State<AppState>,
        Json(payload): Json<PatchUpdateFreq>,
    ) -> Json<Value> {
        let mut guard = blocklist.lock().await;

        guard.update_freq = Duration::from_secs(payload.update_freq);

        Json(json!({
            "status": "success",
        }))
    }

    async fn handle_update_blocklist(
        State(AppState { blocklist, .. }): State<AppState>,
        Json(payload): Json<PatchBlocklist>,
    ) -> Json<Value> {
        let mut guard = blocklist.lock().await;

        match payload.action.as_str() {
            "add" => {
                info!(domain = %payload.domain, "added to blocklist");

                guard.domains.insert(payload.domain.clone());
                guard.user_added.insert(payload.domain.clone());
                guard.user_removed.remove(&payload.domain);

                if guard.save_config().await.is_err() {
                    return Json(json!({
                        "status": "error",
                        "message": "Failed to save config",
                    }));
                }

                Json(json!({
                    "status": "success",
                    "action": "added",
                }))
            }
            "remove" => {
                info!(domain = %payload.domain, "removed from blocklist");

                guard.domains.remove(&payload.domain);
                guard.user_removed.insert(payload.domain.clone());
                guard.user_added.remove(&payload.domain);

                if guard.save_config().await.is_err() {
                    return Json(json!({
                        "status": "error",
                        "message": "Failed to save config",
                    }));
                }

                Json(json!({
                    "status": "success",
                    "action": "removed",
                }))
            }
            _ => Json(json!({
                "status": "error",
                "message": "Invalid action",
            })),
        }
    }

    async fn handle_get_overrides(
        State(AppState { blocklist, .. }): State<AppState>,
    ) -> Json<Value> {
        let guard = blocklist.lock().await;
        let mut added: Vec<&str> = guard.user_added.iter().map(|s| s.as_str()).collect();
        let mut removed: Vec<&str> = guard.user_removed.iter().map(|s| s.as_str()).collect();

        added.sort_unstable();
        removed.sort_unstable();

        Json(json!({ "added": added, "removed": removed }))
    }

    async fn handle_get_sources(
        State(AppState { blocklist, .. }): State<AppState>,
    ) -> Json<Value> {
        let guard = blocklist.lock().await;
        Json(json!({ "sources": guard.sources }))
    }

    async fn handle_patch_sources(
        State(AppState { blocklist, .. }): State<AppState>,
        Json(payload): Json<PatchSources>,
    ) -> Json<Value> {
        let mut guard = blocklist.lock().await;

        match payload.action.as_str() {
            "add" => {
                if guard.sources.iter().any(|s| s.url == payload.url) {
                    return Json(json!({
                        "status": "error",
                        "message": "Source already exists",
                    }));
                }
                guard.sources.push(Source {
                    url: payload.url.clone(),
                    enabled: true,
                });
                info!(url = %payload.url, "added blocklist source");

                match guard.update().await {
                    Ok(_) => {}
                    Err(e) => {
                        error!(error = %e, "failed to update after adding source");
                    }
                }

                if guard.save_config().await.is_err() {
                    return Json(json!({ "status": "error", "message": "Failed to save config" }));
                }

                Json(json!({ "status": "success", "action": "added" }))
            }
            "remove" => {
                guard.sources.retain(|s| s.url != payload.url);
                info!(url = %payload.url, "removed blocklist source");

                match guard.update().await {
                    Ok(_) => {}
                    Err(e) => {
                        error!(error = %e, "failed to update after removing source");
                    }
                }

                if guard.save_config().await.is_err() {
                    return Json(json!({ "status": "error", "message": "Failed to save config" }));
                }

                Json(json!({ "status": "success", "action": "removed" }))
            }
            "toggle" => {
                if let Some(src) = guard.sources.iter_mut().find(|s| s.url == payload.url) {
                    src.enabled = payload.enabled.unwrap_or(!src.enabled);
                    info!(url = %payload.url, enabled = src.enabled, "toggled blocklist source");
                }

                if guard.save_config().await.is_err() {
                    return Json(json!({ "status": "error", "message": "Failed to save config" }));
                }

                if let Err(e) = guard.update().await {
                    error!(error = %e, "update after toggle failed");
                    return Json(
                        json!({ "status": "error", "message": "Failed to update blocklist after toggle" }),
                    );
                }

                drop(guard);

                Json(json!({ "status": "success", "action": "toggled" }))
            }
            _ => Json(json!({ "status": "error", "message": "Invalid action" })),
        }
    }

    async fn handle_analytics_summary(
        State(AppState { analytics, .. }): State<AppState>,
    ) -> Json<Value> {
        match analytics.summary().await {
            Ok(s) => Json(json!(s)),
            Err(e) => {
                error!(error = %e, "analytics summary failed");
                Json(json!({ "error": "failed to fetch analytics" }))
            }
        }
    }

    async fn handle_top_blocked(
        State(AppState { analytics, .. }): State<AppState>,
    ) -> Json<Value> {
        match analytics.top_blocked(10).await {
            Ok(rows) => Json(json!({ "domains": rows })),
            Err(e) => {
                error!(error = %e, "top_blocked query failed");
                Json(json!({ "error": "failed to fetch top blocked" }))
            }
        }
    }

    async fn handle_top_queried(
        State(AppState { analytics, .. }): State<AppState>,
    ) -> Json<Value> {
        match analytics.top_queried(10).await {
            Ok(rows) => Json(json!({ "domains": rows })),
            Err(e) => {
                error!(error = %e, "top_queried query failed");
                Json(json!({ "error": "failed to fetch top queried" }))
            }
        }
    }
}
