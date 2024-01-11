mod db;
mod service;
mod md5_handler;
mod visit_handler;
mod dandanplay_handler;

use mimalloc::MiMalloc;

use axum::http::Method;
use axum::{
  http::{HeaderValue, StatusCode},
  routing::{get, post},
  Router,
};
use std::collections::HashSet;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

use db::setup_db;
use md5_handler::{get_md5, post_md5};
use visit_handler::post_visit;
use dandanplay_handler::{proxy_get_dandanplay_comment, proxy_post_dandanplay_match};

// TODO: split state to immutable & mutable ones to avoid frequently acuire lock
#[derive(Clone)]
pub struct AppState {
  dbpath: String,
  allowed_hosts: Vec<String>,
  processing_files: HashSet<String>,
}

pub type SharedState = Arc<RwLock<AppState>>;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn create_app() -> Router {
  let dbpath = env::var("DANMAKUHUB_DB_PATH").unwrap_or("danmakuhub.db".to_string());
  let allowed_hosts = env::var("DANMAKUHUB_ALLOWED_LINKS")
    .unwrap_or("".to_string())
    .split(";")
    .map(|s| s.to_string())
    .collect();

  let cors_origins: Vec<HeaderValue> = env::var("DANMAKUHUB_CORS_ORIGINS")
    .unwrap_or("http://localhost:5173".to_string())
    .split(";")
    .map(|s| HeaderValue::from_str(s).unwrap())
    .collect();

  let setup_dbpath = dbpath.clone();

  let state = Arc::new(RwLock::new(AppState {
    dbpath,
    allowed_hosts,
    processing_files: HashSet::new(),
  }));

  let cors = CorsLayer::new()
    .allow_methods([Method::POST, Method::GET])
    .allow_origin(cors_origins);

  let app = Router::new()
    .route("/danmakuhub/md5", post(post_md5))
    .route("/danmakuhub/md5", get(get_md5))
    .route("/danmakuhub/visit", post(post_visit))
    .route("/danmakuhub/dandanplay/comment", get(proxy_get_dandanplay_comment))
    .route("/danmakuhub/dandanplay/match", post(proxy_post_dandanplay_match))
    .layer(cors)
    .route("/healthz", get(health))
    .route("/danmakuhub/healthz", get(health))
    .with_state(Arc::clone(&state));

  setup_db(setup_dbpath.as_str());

  app
}

#[tokio::main]
async fn main() {
  env_logger::init();

  let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
  let listener = tokio::net::TcpListener::bind(addr).await.unwrap();  let app = create_app();

  tracing::info!("listening on {}", addr);

  let service = app.into_make_service();

  axum::serve(listener, service)
    .await
    .unwrap();
}

pub async fn health() -> StatusCode {
  StatusCode::OK
}
