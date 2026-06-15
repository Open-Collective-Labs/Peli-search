use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use pelisearch_core::engine::SearchEngine;
use tokio::sync::Mutex;

use crate::handlers::openapi::OpenApiSpec;

/// Shared application state, accessible from all handlers.
pub struct AppState {
    /// The search engine instance (wrapped in a mutex for interior mutability).
    pub engine: Mutex<SearchEngine>,
    /// Runtime metrics counters.
    pub metrics: Metrics,
    /// OpenAPI specification as cached JSON.
    pub openapi_spec: Option<OpenApiSpec>,
}

impl AppState {
    /// Create a new `AppState`, opening the persistent engine at `data_dir`.
    pub async fn new(data_dir: impl Into<std::path::PathBuf>) -> Result<Self, String> {
        let engine = SearchEngine::open(data_dir).map_err(|e| format!("failed to open engine: {e}"))?;

        let openapi_spec = Self::load_openapi();

        Ok(Self {
            engine: Mutex::new(engine),
            metrics: Metrics::new(),
            openapi_spec,
        })
    }

    fn load_openapi() -> Option<OpenApiSpec> {
        let yaml = include_str!("../../../docs/openapi.yaml");
        match OpenApiSpec::from_yaml(yaml) {
            Ok(spec) => Some(spec),
            Err(e) => {
                eprintln!("WARNING: failed to load OpenAPI spec: {e}");
                None
            }
        }
    }
}

/// Type alias for the shared state used across handlers.
pub type SharedState = Arc<AppState>;

/// Runtime metrics counters.
#[derive(Debug)]
pub struct Metrics {
    /// Total HTTP requests handled.
    pub request_count: AtomicU64,
    /// Total search queries executed.
    pub search_count: AtomicU64,
    /// Accumulated latency in nanoseconds across all requests.
    pub total_latency_ns: AtomicU64,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            request_count: AtomicU64::new(0),
            search_count: AtomicU64::new(0),
            total_latency_ns: AtomicU64::new(0),
        }
    }

    pub fn inc_requests(&self) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_search(&self) {
        self.search_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_latency(&self, ns: u64) {
        self.total_latency_ns.fetch_add(ns, Ordering::Relaxed);
    }

    /// Snapshot of all current metric values.
    pub fn snapshot(&self, engine: &SearchEngine) -> MetricSnapshot {
        MetricSnapshot {
            request_count: self.request_count.load(Ordering::Relaxed),
            search_count: self.search_count.load(Ordering::Relaxed),
            total_latency_ns: self.total_latency_ns.load(Ordering::Relaxed),
            document_count: engine.total_document_count(),
            index_count: engine.list_indexes().len() as u64,
        }
    }
}

/// A point-in-time copy of all metric values.
#[derive(Debug, serde::Serialize)]
pub struct MetricSnapshot {
    pub request_count: u64,
    pub search_count: u64,
    pub total_latency_ns: u64,
    pub document_count: u64,
    pub index_count: u64,
}
