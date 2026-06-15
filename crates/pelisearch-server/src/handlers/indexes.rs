use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use pelisearch_core::engine::IndexInfo;
use pelisearch_core::error::SearchError;

use crate::state::SharedState;

#[derive(Debug, Serialize)]
pub struct IndexListResponse {
    pub indexes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateIndexRequest {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct IndexCreatedResponse {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
}

/// GET /health
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
    })
}

/// GET /indexes
pub async fn list_indexes(
    State(state): State<SharedState>,
) -> Result<Json<IndexListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().await;
    let indexes = engine.list_indexes();
    Ok(Json(IndexListResponse { indexes }))
}

/// GET /indexes/:name
pub async fn get_index(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> Result<Json<IndexInfo>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().await;
    let info = engine
        .get_index_info(&name)
        .map_err(|e| not_found_error(e.to_string()))?;
    Ok(Json(info))
}

/// POST /indexes
pub async fn create_index(
    State(state): State<SharedState>,
    Json(payload): Json<CreateIndexRequest>,
) -> Result<(StatusCode, Json<IndexCreatedResponse>), (StatusCode, Json<ErrorResponse>)> {
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        return Err(bad_request_error("index name must not be empty".into()));
    }
    
    // Validate index name (alphanumeric, dashes, underscores, 1-128 chars)
    if name.len() > 128 || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(bad_request_error("index name must be 1-128 characters and contain only alphanumeric characters, dashes, and underscores".into()));
    }

    let mut engine = state.engine.write().await;
    engine
        .create_index(&name)
        .map_err(handle_create_error)?;
    Ok((
        StatusCode::CREATED,
        Json(IndexCreatedResponse { name }),
    ))
}

/// DELETE /indexes/:name
pub async fn delete_index(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let mut engine = state.engine.write().await;
    engine
        .delete_index(&name)
        .map_err(|e| not_found_error(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

fn handle_create_error(e: SearchError) -> (StatusCode, Json<ErrorResponse>) {
    match e {
        SearchError::IndexAlreadyExists(name) => {
            conflict_error(format!("index '{name}' already exists"))
        }
        _ => bad_request_error(e.to_string())
    }
}

fn bad_request_error(msg: String) -> (StatusCode, Json<ErrorResponse>) {
    (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: msg }))
}

fn conflict_error(msg: String) -> (StatusCode, Json<ErrorResponse>) {
    (StatusCode::CONFLICT, Json(ErrorResponse { error: msg }))
}

fn not_found_error(msg: String) -> (StatusCode, Json<ErrorResponse>) {
    (StatusCode::NOT_FOUND, Json(ErrorResponse { error: msg }))
}
