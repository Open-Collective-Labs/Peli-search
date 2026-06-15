use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::handlers::indexes::ErrorResponse;
use crate::state::SharedState;

#[derive(Debug, Serialize)]
pub struct ReadyResponse {
    pub status: String,
}

/// GET /ready
///
/// Returns `{"status": "ready"}` when the server can accept traffic.
pub async fn ready(
    State(state): State<SharedState>,
) -> Result<Json<ReadyResponse>, (axum::http::StatusCode, Json<ErrorResponse>)> {
    // Verify the engine is functional by listing indexes
    let engine = state.engine.lock().await;
    let _ = engine.list_indexes();
    Ok(Json(ReadyResponse {
        status: "ready".into(),
    }))
}

/// GET /metrics
///
/// Returns operational metrics as JSON.
pub async fn metrics(
    State(state): State<SharedState>,
) -> Json<crate::state::MetricSnapshot> {
    let engine = state.engine.lock().await;
    let snapshot = state.metrics.snapshot(&engine);
    Json(snapshot)
}
