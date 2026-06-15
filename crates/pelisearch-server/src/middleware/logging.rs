use std::time::Duration;

use axum::body::Body;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use tracing::info;

/// Middleware that logs each request with method, path, status, duration,
/// and request ID (from the `X-Request-Id` response header).
///
/// Must be placed **outside** the request-id middleware so that the
/// `X-Request-Id` header has already been set on the response when
/// this middleware observes it.
pub async fn request_logger(req: Request<Body>, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = std::time::Instant::now();

    let response = next.run(req).await;

    let elapsed = start.elapsed();
    let status = response.status();

    let request_id = response
        .headers()
        .get("X-Request-Id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    log_request(&method, &uri, status, elapsed, request_id);

    response
}

fn log_request(
    method: &axum::http::Method,
    uri: &axum::http::Uri,
    status: axum::http::StatusCode,
    elapsed: Duration,
    request_id: &str,
) {
    info!(
        request_id = request_id,
        method = %method,
        path = %uri,
        status = status.as_u16(),
        duration_ms = elapsed.as_millis(),
        "request completed",
    );
}
