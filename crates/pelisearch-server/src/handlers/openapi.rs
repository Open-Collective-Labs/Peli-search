use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::Json;
use serde_json::Value;

use crate::handlers::indexes::ErrorResponse;
use crate::state::SharedState;

/// OpenAPI specification in JSON format, cached at startup.
#[derive(Clone)]
pub struct OpenApiSpec(Arc<String>);

impl OpenApiSpec {
    pub fn from_yaml(yaml: &str) -> Result<Self, String> {
        let parsed: Value = serde_yaml::from_str(yaml)
            .map_err(|e| format!("failed to parse OpenAPI YAML: {e}"))?;
        let json = serde_json::to_string_pretty(&parsed)
            .map_err(|e| format!("failed to serialize OpenAPI JSON: {e}"))?;
        Ok(Self(Arc::new(json)))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// GET /openapi.json
///
/// Returns the full OpenAPI 3.1 specification as JSON.
pub async fn openapi_json(
    State(state): State<SharedState>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let spec = state
        .openapi_spec
        .as_ref()
        .ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "OpenAPI spec not loaded".into(),
                }),
            )
        })?;

    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        spec.as_str().to_owned(),
    )
        .into_response())
}

/// GET /docs
///
/// Swagger UI HTML page for interactive API documentation.
pub async fn docs_page() -> Html<&'static str> {
    Html(include_str!("../../../../docs/swagger-ui.html"))
}
