use std::sync::Arc;
use std::time::Duration;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode, Uri};
use axum::{Json, Router};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use rust_embed::RustEmbed;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::Mutex;
use crate::blocklist::Blocklist;

#[derive(RustEmbed)]
#[folder = "frontend/dist/"]
struct Assets;

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

impl Server {
    pub async fn run(blocklist: Arc<Mutex<Blocklist>>) {
        let app = Router::new()
            .route("/check_blocklist/{domain}", get(Self::check_blocklist))
            .route("/set_update_freq", post(Self::handle_update_freq))
            .route("/update_blocklist", post(Self::handle_update_blocklist))
            .with_state(blocklist)
            .fallback(Self::frontend);

        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
        axum::serve(listener, app).await.unwrap();
    }

    async fn frontend(uri: Uri) -> Response {
        let path = uri.path().trim_start_matches('/');

        match Assets::get(path) {
            Some(content) => {
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                ([(header::CONTENT_TYPE, mime.as_ref().to_owned())], content.data).into_response()
            }
            None => match Assets::get("index.html") {
                Some(index) => Html(index.data).into_response(),
                None => StatusCode::NOT_FOUND.into_response(),
            },
        }
    }

    async fn check_blocklist(Path(domain): Path<String>, State(blocklist): State<Arc<Mutex<Blocklist>>>) -> Json<Value> {
        let guard = blocklist.lock().await;

        Json(json!({
            "blocked": guard.check(&*domain),
        }))
    }

    async fn handle_update_freq(State(blocklist): State<Arc<Mutex<Blocklist>>>, Json(payload): Json<PatchUpdateFreq>) -> Json<Value> {
        let mut guard = blocklist.lock().await;

        guard.update_freq = Duration::from_secs(payload.update_freq);

        Json(json!({
            "status": "success",
        }))
    }

    async fn handle_update_blocklist(State(blocklist): State<Arc<Mutex<Blocklist>>>, Json(payload): Json<PatchBlocklist>) -> Json<Value> {
        let mut guard = blocklist.lock().await;

        match payload.action.as_str() {
            "add" => {
                println!("Added {} to blocklist", payload.domain);

                guard.domains.insert(payload.domain.clone());
                guard.user_added.insert(payload.domain);

                Json(json!({
                    "status": "success",
                    "action": "added",
                }))
            },
            "remove" => {
                println!("Removed {} from blocklist", payload.domain);

                guard.domains.remove(&payload.domain);
                guard.user_removed.insert(payload.domain);

                Json(json!({
                    "status": "success",
                    "action": "removed",
                }))
            },
            _ => {
                Json(json!({
                    "status": "error",
                    "message": "Invalid action",
                }))
            }
        }
    }
}
