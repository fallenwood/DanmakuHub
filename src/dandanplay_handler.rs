use axum::{body::Body, extract::{Json, Query, State}, http::{HeaderMap, HeaderValue, StatusCode}, response::IntoResponse};
use serde::{Deserialize, Serialize};

use crate::SharedState;

#[derive(Deserialize)]
pub struct ProxyDandanPlayCommentRequest {
  pub episode_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProxyDandanPlayMatchRequest {
  #[serde(rename = "fileName")]
  pub file_name: Option<String>,
  #[serde(rename = "fileSize")]
  pub file_size: Option<i64>,
  #[serde(rename = "fileHash")]
  pub file_hash: Option<String>,
  #[serde(rename = "matchMode")]
  pub match_mode: Option<String>,
}

async fn add_dandanplay_headers(headers: &mut HeaderMap, state: &SharedState) {
  let app_id = state.read().await.app_id.clone();
  let app_secret = state.read().await.app_secret.clone();

  headers.insert("X-AppId", HeaderValue::from_str(&app_id).unwrap());
  headers.insert("X-AppSecret", HeaderValue::from_str(&app_secret).unwrap());
}

pub async fn proxy_post_dandanplay_match(
  State(state): State<SharedState>,
  Json(req): Json<ProxyDandanPlayMatchRequest>,
) -> impl IntoResponse {
  let mut headers = HeaderMap::new();
  let uri = "https://api.dandanplay.net/api/v2/match";

  let client = reqwest::Client::new();
  let reqwest_response = client.post(uri).json(&req).send().await.unwrap();

  // TODO: iter all headers
  if let Some(content_type) = reqwest_response.headers().get("content-type") {
    headers.insert("content-type", content_type.clone().to_str().unwrap().parse().unwrap());
  }

  add_dandanplay_headers(&mut headers, &state).await;

  let stream = reqwest_response.bytes_stream();

  return (
    StatusCode::OK,
    headers,
    Body::from_stream(stream));
}

pub async fn proxy_get_dandanplay_comment(
  State(state): State<SharedState>,
  Query(query): Query<ProxyDandanPlayCommentRequest>,
) -> impl IntoResponse {
  // https://api.dandanplay.net/api/v2/comment/${episode_id}?withRelated=true&chConvert=0

  let mut headers = HeaderMap::new();

  if let Some(episode_id) = query.episode_id {
    let uri = format!("https://api.dandanplay.net/api/v2/comment/{episode_id}?withRelated=true&chConvert=0");

    let client = reqwest::Client::new();
    let reqwest_response = client.get(&uri).send().await.unwrap();

    // TODO: iter all headers
    if let Some(content_type) = reqwest_response.headers().get("content-type") {
      headers.insert("content-type", content_type.clone().to_str().unwrap().parse().unwrap());
    }

    add_dandanplay_headers(&mut headers, &state).await;

    let stream = reqwest_response.bytes_stream();

    return (
      StatusCode::OK,
      headers,
      Body::from_stream(stream));
  } else {
    return (
      StatusCode::BAD_REQUEST,
      headers,
      Body::from(""));
  }
}
