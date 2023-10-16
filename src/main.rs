use axum::extract::State;
use axum::Json;
use mimalloc::MiMalloc;

use axum::http::Method;
use axum::{
  extract::Query,
  http::{HeaderValue, StatusCode},
  response::IntoResponse,
  routing::{get, post},
  Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::collections::HashSet;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[derive(Deserialize)]
pub struct GetMd5Request {
  pub link: Option<String>,
  pub filename: Option<String>,
}

#[derive(Serialize)]
pub struct GetMd5Response {
  pub hash: String,
}

// TODO: split state to immutable & mutable ones to avoid frequently acuire lock
#[derive(Clone)]
struct AppState {
  dbpath: String,
  allowed_hosts: Vec<String>,
  processing_files: HashSet<String>,
}

type SharedState = Arc<RwLock<AppState>>;

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
  let app = create_app();

  tracing::info!("listening on {}", addr);

  axum::Server::bind(&addr)
    .serve(app.into_make_service())
    .await
    .unwrap();
}

pub async fn health() -> StatusCode {
  StatusCode::OK
}

fn setup_db(dbpath: &str) {
  let connection = sqlite::open(dbpath).unwrap();

  let query = "create table if not exists Hashes(
    Id INTEGER PRIMARY KEY AUTOINCREMENT,
    Filename VARCHAR(1024) NOT NULL UNIQUE,
    Hash VARCHAR(1024) NOT NULL)
  ";

  connection.execute(query).unwrap();

  tracing::info!("db setup done");
}

fn insert_db(dbpath: &str, filename: &str, hash: &str) -> Option<sqlite::Error> {
  let connection = sqlite::open(dbpath).unwrap();

  let query = "insert into Hashes (Filename, Hash)
  values (?, ?)
  on conflict (Filename) do
  update set Hash = ?;";
  let mut statement = connection.prepare(query).unwrap();
  statement.bind((1, filename)).unwrap();
  statement.bind((2, hash)).unwrap();
  statement.bind((3, hash)).unwrap();

  statement.next().err()
}

fn query_db(dbpath: &str, filename: &str) -> Option<String> {
  let connection = sqlite::open(dbpath).unwrap();

  let query = "select hash from Hashes where Filename=? limit 1";
  let mut statement = connection.prepare(query).unwrap();
  statement.bind((1, filename)).unwrap();

  if let Ok(state) = statement.next() {
    if state == sqlite::State::Row {
      if let Ok(hash) = statement.read::<String, usize>(0) {
        return Some(hash);
      }
    }
  }

  None
}

async fn download_16m(link: &str) -> Option<String> {
  let client = reqwest::Client::new();

  tracing::debug!("Start download {}", link);

  let resp = client
    .get(link)
    .header("Range", "bytes=0-16777215")
    .send()
    .await;

  match resp {
    Ok(resp) => {
      tracing::debug!("Successfully download {}", link);

      let body = resp.bytes().await.unwrap();
      let md5 = md5::compute(body);
      let md5_str = format!("{:x}", md5);

      tracing::debug!("Successfully calculating md5 {}", link);

      return Some(md5_str);
    }

    Err(e) => {
      tracing::error!("download failed for {}: {}", link, e);
      return None;
    }
  }
}

async fn handle_md5_request(
  link: String,
  filename: String,
  state: SharedState,
  run_background_fetch: bool,
) -> axum::response::Response {
  let dbpath = state.read().await.dbpath.clone();

  if let Some(hash) = query_db(dbpath.as_str(), filename.as_str()) {
    let response = GetMd5Response { hash };
    return Json(response).into_response();
  }

  if run_background_fetch {
    let _ = tokio::spawn(async move {
      if state.read().await.processing_files.contains(filename.as_str()) {
        tracing::info!("skip processing file {}", filename);
        return;
      }

      state.write().await.processing_files.insert(filename.clone());

      let Some(md5) = download_16m(link.as_str()).await else {
        tracing::error!("failed to download md5 for {}", filename);
        return;
      };

      if let Some(err) = insert_db(dbpath.as_str(), filename.as_str(), md5.as_str()) {
        tracing::error!("insert failed for {}: {}", filename, err);
      } else {
        tracing::info!("Successfully insert db for {}", filename);
      }
    });
  }

  StatusCode::NOT_FOUND.into_response()
}

async fn get_md5(
  Query(query): Query<GetMd5Request>,
  State(state): State<SharedState>,
) -> impl IntoResponse {
  let Some(filename) = query.filename else {
    return StatusCode::BAD_REQUEST.into_response();
  };

  handle_md5_request("".to_string(), filename, state, false).await
}

async fn post_md5(
  Query(query): Query<GetMd5Request>,
  State(state): State<SharedState>,
) -> impl IntoResponse {
  let (Some(link), Some(filename)) = (query.link, query.filename) else {
    return StatusCode::BAD_REQUEST.into_response();
  };

  if !state
    .read()
    .await
    .allowed_hosts
    .iter()
    .any(|allowed| link.starts_with(allowed))
  {
    return StatusCode::BAD_REQUEST.into_response();
  }

  handle_md5_request(link, filename, state, true).await
}
