use axum::{extract::{Query, State}, response::IntoResponse, Json, http::StatusCode};
use serde::Serialize;

use crate::{md5_handler::GetMd5Request, SharedState, db::update_visits};


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
