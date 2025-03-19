use axum::{
  Json,
  extract::{Query, State},
  http::StatusCode,
  response::IntoResponse,
};
use serde::Serialize;

use crate::{SharedState, db::update_visits, md5_handler::GetMd5Request};

#[derive(Serialize)]
pub struct GetVisitResponse {
  pub visits: i64,
}

pub async fn post_visit(
  Query(query): Query<GetMd5Request>,
  State(state): State<SharedState>,
) -> impl IntoResponse {
  let Some(filename) = query.filename else {
    return StatusCode::BAD_REQUEST.into_response();
  };

  let dbpath = state.read().await.dbpath.clone();

  let visits = update_visits(dbpath.as_str(), filename.as_str()).unwrap_or(-1);
  return Json(GetVisitResponse { visits }).into_response();
}
