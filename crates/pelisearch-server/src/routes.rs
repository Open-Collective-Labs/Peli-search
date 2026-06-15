use std::str::FromStr;

use axum::http::{HeaderName, Method};
use axum::middleware;
use axum::routing::{delete, get, post};
use axum::Router;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};

use crate::config::ServerConfig;
use crate::handlers::{documents, indexes, monitoring, openapi};
use crate::middleware::{auth, logging, metrics, rate_limit, request_id};
use crate::state::SharedState;

/// Build the application router with all registered routes.
pub fn build_router(state: SharedState, config: &ServerConfig) -> Router {
    // ---- S-6: CORS ----
    let cors = if config.cors_enabled {
        let origins: Vec<_> = config
            .cors_origins
            .iter()
            .map(|o| o.parse().unwrap())
            .collect();
        let methods: Vec<_> = config
            .cors_methods
            .iter()
            .map(|m| Method::from_bytes(m.as_bytes()).unwrap())
            .collect();
        let headers: Vec<_> = config
            .cors_headers
            .iter()
            .map(|h| HeaderName::from_str(h).unwrap())
            .collect();
        let mut layer = CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_methods(AllowMethods::list(methods))
            .allow_headers(AllowHeaders::list(headers));
        if config.cors_credentials {
            layer = layer.allow_credentials(true);
        }
        Some(layer)
    } else {
        // Default permissive CORS (all origins)
        Some(CorsLayer::permissive())
    };

    let mut router = Router::new()
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
        .route("/indexes/:name/search", post(documents::search));

    // Apply middleware layers
    // Order: innermost first

    // S-5: Rate limiting (if enabled)
    if config.rate_limit_enabled {
        router = router.layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limit::rate_limit,
        ));
    }

    // S-1: Authentication (if enabled)
    if config.api_key.is_some() && config.auth_enabled {
        router = router.layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_auth,
        ));
    }

    // Metrics, request ID, logging (always applied)
    router = router
        .layer(middleware::from_fn_with_state(
            state.clone(),
            metrics::track_metrics,
        ))
        .layer(middleware::from_fn(request_id::add_request_id))
        .layer(middleware::from_fn(logging::request_logger));

    // CORS (applied as outermost layer)
    if let Some(cors) = cors {
        router = router.layer(cors);
    }

    // Shared state
    router.with_state(state)
}
