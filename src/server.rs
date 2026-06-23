use std::sync::Arc;
use axum::Router;
use axum::routing::get;
use tokio::sync::Mutex;
use crate::blocklist::Blocklist;

pub struct Server;

impl Server {
    pub async fn run(blocklist: Arc<Mutex<Blocklist>>) {
        let app = Router::new()
            .route("/", get(|| async { "Hello, World!" }))
            .with_state(blocklist);

        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
        axum::serve(listener, app).await.unwrap();
    }
}
