use std::sync::Arc;
use std::time::Duration;
use axum::extract::State;
use axum::{Json, Router};
use axum::routing::get;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::Mutex;
use crate::blocklist::Blocklist;

pub struct Server;

#[derive(Deserialize)]
struct PatchBlocklist {
    update_freq: Option<u64>,
    domains: Option<Vec<String>>,
}

impl Server {
    pub async fn run(blocklist: Arc<Mutex<Blocklist>>) {
        let app = Router::new()
            .route("/", get(|| async { "Hello, World!" }))
            .route("/blocklist", get(Self::get_blocklist).patch(Self::patch_blocklist))
            .with_state(blocklist);

        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
        axum::serve(listener, app).await.unwrap();
    }

    async fn get_blocklist(State(blocklist): State<Arc<Mutex<Blocklist>>>) -> Json<Value> {
        let guard = blocklist.lock().await;
        Json(json!({
            "update_freq": guard.update_freq.as_secs(),
            "last_update": guard.last_update.elapsed().as_secs(),
            "domains": guard.domains,
        }))
    }

    async fn patch_blocklist(Json(payload): Json<PatchBlocklist>, State(blocklist): State<Arc<Mutex<Blocklist>>>) -> Json<Value> {
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
