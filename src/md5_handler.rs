use axum::{Json, extract::{Query, State}, response::IntoResponse, http::StatusCode};
use serde::{Deserialize, Serialize};

use crate::{SharedState, db::{query_db, insert_db}, service::download_16m};


#[derive(Deserialize)]
pub struct GetMd5Request {
  pub link: Option<String>,
  pub filename: Option<String>,
}

#[derive(Serialize)]
pub struct GetMd5Response {
  pub hash: String,
}

pub async fn handle_md5_request(
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
      if state
        .read()
        .await
        .processing_files
        .contains(filename.as_str())
      {
        tracing::info!("skip processing file {}", filename);
        return;
      }

      state
        .write()
        .await
        .processing_files
        .insert(filename.clone());

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

pub async fn get_md5(
  Query(query): Query<GetMd5Request>,
  State(state): State<SharedState>,
) -> impl IntoResponse {
  let Some(filename) = query.filename else {
    return StatusCode::BAD_REQUEST.into_response();
  };

  handle_md5_request("".to_string(), filename, state, false).await
}

pub async fn post_md5(
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
