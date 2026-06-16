use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{ConnectInfo, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use tokio::sync::Mutex;
use tokio::time::Instant;

use crate::state::SharedState;

struct Bucket {
    tokens: u64,
    reset_at: Instant,
}

/// Per-IP rate limiter using a token bucket approach.
///
/// Each IP gets a bucket with `max_tokens` capacity that refills every minute.
pub struct RateLimiter {
    buckets: Mutex<HashMap<String, Bucket>>,
    max_tokens: u64,
    window_secs: u64,
}

impl RateLimiter {
    pub fn new(max_tokens: u64) -> Arc<Self> {
        Arc::new(Self {
            buckets: Mutex::new(HashMap::new()),
            max_tokens,
            window_secs: 60,
        })
    }

    async fn check(&self, ip: &str) -> bool {
        let mut buckets = self.buckets.lock().await;
        let now = Instant::now();

        let bucket = buckets.entry(ip.to_string()).or_insert(Bucket {
            tokens: self.max_tokens,
            reset_at: now,
        });

        if now >= bucket.reset_at {
            bucket.tokens = self.max_tokens;
            bucket.reset_at = now + tokio::time::Duration::from_secs(self.window_secs);
        }

        if bucket.tokens > 0 {
            bucket.tokens -= 1;
            true
        } else {
            false
        }
    }
}

/// Rate limiting middleware.
pub async fn rate_limit(
    State(state): State<SharedState>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let rate_limiter = match state.rate_limiter.as_ref() {
        Some(rl) => rl.clone(),
        None => return Ok(next.run(request).await),
    };

    let ip = request
        .headers()
        .get("X-Forwarded-For")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next().map(|s| s.trim().to_string()))
        .or_else(|| {
            request
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|ci| ci.0.ip().to_string())
        })
        .unwrap_or_else(|| "unknown".into());

    if !rate_limiter.check(&ip).await {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "rate limit exceeded, try again later".into(),
        ));
    }

    Ok(next.run(request).await)
}
