use axum::middleware;
use axum::routing::{delete, get, post};
use axum::Router;

use crate::handlers::{documents, indexes, monitoring, openapi};
use crate::middleware::{logging, metrics, request_id};
use crate::state::SharedState;

/// Build the application router with all registered routes.
pub fn build_router(state: SharedState) -> Router {
    Router::new()
        // API documentation
        .route("/openapi.json", get(openapi::openapi_json))
        .route("/docs", get(openapi::docs_page))
        // Health / Readiness / Metrics
        .route("/health", get(indexes::health))
        .route("/ready", get(monitoring::ready))
        .route("/metrics", get(monitoring::metrics))
        // Index management
        .route("/indexes", get(indexes::list_indexes))
        .route("/indexes", post(indexes::create_index))
        .route("/indexes/:name", get(indexes::get_index))
        .route("/indexes/:name", delete(indexes::delete_index))
        // Documents
        .route("/indexes/:name/documents", post(documents::add_document))
        .route(
            "/indexes/:name/documents/bulk",
            post(documents::bulk_add_documents),
        )
        .route(
            "/indexes/:name/documents/:id",
            get(documents::get_document),
        )
        .route(
            "/indexes/:name/documents/:id",
            delete(documents::delete_document),
        )
        // Search
        .route("/indexes/:name/search", post(documents::search))
        // Middleware — first `.layer()` wraps the router (innermost),
        // subsequent layers wrap outward.
        //
        //  1. metrics      (innermost) — counts requests and latency
        //  2. request_id   (middle)     — sets X-Request-Id on response
        //  3. logging      (outermost)  — reads X-Request-Id from response
        .layer(middleware::from_fn_with_state(
            state.clone(),
            metrics::track_metrics,
        ))
        .layer(middleware::from_fn(request_id::add_request_id))
        .layer(middleware::from_fn(logging::request_logger))
        // Shared state for handlers
        .with_state(state)
}
