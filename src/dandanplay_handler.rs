use axum::{extract::Query, response::IntoResponse, http::{StatusCode, HeaderMap}, body::Body};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ProxyDandanPlayCommentRequest {
  pub episode_id: Option<String>,
}

pub async fn proxy_get_dandanplay_comment(
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
      tracing::warn!("content-type: {}", content_type.clone().to_str().unwrap());
      headers.insert("content-type", content_type.clone().to_str().unwrap().parse().unwrap());
    }

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
