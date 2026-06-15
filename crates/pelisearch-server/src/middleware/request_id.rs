use axum::body::Body;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

/// Middleware that assigns a unique UUID to every request.
///
/// The ID is set as the `X-Request-Id` response header for correlation
/// by outer middleware (e.g. the logging middleware) and downstream clients.
pub async fn add_request_id(req: Request<Body>, next: Next) -> Response {
    let id = uuid::Uuid::new_v4().to_string();

    let response = next.run(req).await;

    let mut response = response;
    response
        .headers_mut()
        .insert("X-Request-Id", id.parse().unwrap());
    response
}
