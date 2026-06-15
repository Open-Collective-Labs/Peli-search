mod config;
mod handlers;
mod middleware;
mod routes;
mod state;

use std::net::SocketAddr;

use tokio::signal;
use tracing::info;

use crate::config::load_config;
use crate::state::AppState;

#[tokio::main]
async fn main() {
    // Load configuration (CLI args override config file)
    let config = match load_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("FATAL: {e}");
            std::process::exit(1);
        }
    };

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("pelisearch_server={},tower_http=info", config.log_level).into()),
        )
        .init();

    // Open the search engine
    let state = match AppState::new(&config.data_path).await {
        Ok(s) => std::sync::Arc::new(s),
        Err(e) => {
            eprintln!("FATAL: {e}");
            std::process::exit(1);
        }
    };

    // Build the router
    let app = routes::build_router(state);

    // Bind and serve
    let addr = SocketAddr::new(
        config.host.parse().expect("invalid host address"),
        config.port,
    );
    info!("listening on {addr}");
    info!("data directory: {}", config.data_path.display());

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

/// Wait for SIGINT or SIGTERM to trigger graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("shutting down gracefully");
}
