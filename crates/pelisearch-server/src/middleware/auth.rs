use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;

use crate::state::SharedState;

/// Authentication middleware — checks `Authorization: Bearer <key>` or
/// `X-API-Key: <key>` headers against the configured API key.
///
/// Health/readiness endpoints are excluded from auth checks.
pub async fn require_auth(
    State(state): State<SharedState>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let key = match state.api_key.as_ref() {
        Some(k) => k,
        None => return Ok(next.run(request).await),
    };

    let path = request.uri().path();
    // Skip auth for health and docs endpoints
    if path == "/health" || path == "/ready" || path == "/docs" || path == "/openapi.json" {
        return Ok(next.run(request).await);
    }

    let provided = request
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .or_else(|| {
            request
                .headers()
                .get("X-API-Key")
                .and_then(|v| v.to_str().ok())
        });

    match provided {
        Some(token) if token == key.as_ref() => Ok(next.run(request).await),
        _ => Err((
            StatusCode::UNAUTHORIZED,
            "missing or invalid API key".into(),
        )),
    }
}
