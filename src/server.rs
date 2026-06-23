use std::sync::Arc;
use std::time::Duration;
use axum::extract::State;
use axum::http::{header, StatusCode, Uri};
use axum::{Json, Router};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
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
struct PatchBlocklist {
    update_freq: Option<u64>,
    domains: Option<Vec<String>>,
}

impl Server {
    pub async fn run(blocklist: Arc<Mutex<Blocklist>>) {
        let app = Router::new()
            .route("/blocklist", get(Self::get_blocklist).post(Self::patch_blocklist))
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

    async fn get_blocklist(State(blocklist): State<Arc<Mutex<Blocklist>>>) -> Json<Value> {
        let guard = blocklist.lock().await;
        Json(json!({
            "update_freq": guard.update_freq.as_secs(),
            "last_update": guard.last_update.elapsed().as_secs(),
            "domains": guard.domains.iter().cloned().collect::<Vec<String>>(),
        }))
    }

    async fn patch_blocklist(State(blocklist): State<Arc<Mutex<Blocklist>>>, Json(payload): Json<PatchBlocklist>) -> Json<Value> {
        let mut guard = blocklist.lock().await;

        if let Some(update_freq) = payload.update_freq {
            guard.update_freq = Duration::from_secs(update_freq);
        }

        if let Some(domains) = payload.domains {
            for domain in domains {
                guard.domains.insert(domain);
            }
        }

        Json(json!({
            "status": "success"
        }))
    }
}
