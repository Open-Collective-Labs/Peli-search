use axum::body::Body;
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;

use crate::state::SharedState;

/// Middleware that records request count and accumulated latency.
///
/// Must be applied together with an `AppState` that has a `metrics` field.
pub async fn track_metrics(
    State(state): State<SharedState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let start = std::time::Instant::now();

    state.metrics.inc_requests();

    let response = next.run(req).await;

    let elapsed = start.elapsed();
    state.metrics.add_latency(elapsed.as_nanos() as u64);

    response
}
