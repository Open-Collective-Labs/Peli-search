use std::path::PathBuf;

use clap::Parser;
use serde::Deserialize;

/// Server configuration that can be loaded from a TOML/JSON file or CLI args.
///
/// CLI flags override values from the config file.
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// Host address to bind to.
    #[serde(default = "default_host")]
    pub host: String,

    /// Port to listen on.
    #[serde(default = "default_port")]
    pub port: u16,

    /// Data directory for persistent storage.
    #[serde(default = "default_data_path")]
    pub data_path: PathBuf,

    /// Log level (e.g. "info", "debug", "warn", "error").
    #[serde(default = "default_log_level")]
    pub log_level: String,

    // ---- S-1: API Key Authentication ----

    /// API key for Bearer / X-API-Key authentication.
    #[serde(default)]
    pub api_key: Option<String>,

    /// When true and `api_key` is set, auth is enforced.
    #[serde(default = "default_true")]
    pub auth_enabled: bool,

    // ---- S-5: Rate Limiting ----

    /// Enable per-IP rate limiting.
    #[serde(default)]
    pub rate_limit_enabled: bool,

    /// Max requests per minute per IP.
    #[serde(default = "default_rpm")]
    pub rate_limit_requests_per_minute: u64,

    // ---- S-6: CORS ----

    /// Enable CORS support.
    #[serde(default)]
    pub cors_enabled: bool,

    /// Allowed origins (default: ["*"]).
    #[serde(default = "default_cors_origins")]
    pub cors_origins: Vec<String>,

    /// Allowed methods (default: GET, POST, DELETE, OPTIONS).
    #[serde(default = "default_cors_methods")]
    pub cors_methods: Vec<String>,

    /// Allowed headers (default: content-type, authorization, x-api-key).
    #[serde(default = "default_cors_headers")]
    pub cors_headers: Vec<String>,

    /// Allow credentials (cookies, auth headers).
    #[serde(default)]
    pub cors_credentials: bool,
}

fn default_true() -> bool {
    true
}

fn default_rpm() -> u64 {
    60
}

fn default_host() -> String {
    "127.0.0.1".into()
}

fn default_port() -> u16 {
    7700
}

fn default_data_path() -> PathBuf {
    PathBuf::from("./data")
}

fn default_log_level() -> String {
    "info".into()
}

fn default_cors_origins() -> Vec<String> {
    vec!["*".into()]
}

fn default_cors_methods() -> Vec<String> {
    vec!["GET".into(), "POST".into(), "DELETE".into(), "OPTIONS".into()]
}

fn default_cors_headers() -> Vec<String> {
    vec!["content-type".into(), "authorization".into(), "x-api-key".into()]
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            data_path: default_data_path(),
            log_level: default_log_level(),
            api_key: None,
            auth_enabled: true,
            rate_limit_enabled: false,
            rate_limit_requests_per_minute: 60,
            cors_enabled: false,
            cors_origins: default_cors_origins(),
            cors_methods: default_cors_methods(),
            cors_headers: default_cors_headers(),
            cors_credentials: false,
        }
    }
}

/// CLI overrides that take precedence over config file values.
#[derive(Debug, Parser)]
#[command(name = "pelisearch-server", version, about)]
struct CliArgs {
    /// Path to a TOML or JSON config file.
    #[arg(long, short = 'c')]
    config: Option<PathBuf>,

    /// Host address to bind to (overrides config file).
    #[arg(long)]
    host: Option<String>,

    /// Port to listen on (overrides config file).
    #[arg(long)]
    port: Option<u16>,

    /// Data directory for persistent storage (overrides config file).
    #[arg(long)]
    data_path: Option<PathBuf>,

    /// Log level (overrides config file).
    #[arg(long)]
    log_level: Option<String>,

    /// API key for authentication (overrides config file).
    #[arg(long)]
    api_key: Option<String>,

    /// Enable/disable authentication (overrides config file).
    #[arg(long)]
    auth_enabled: Option<bool>,

    /// Enable/disable rate limiting (overrides config file).
    #[arg(long)]
    rate_limit_enabled: Option<bool>,

    /// Requests per minute limit (overrides config file).
    #[arg(long)]
    rate_limit_requests_per_minute: Option<u64>,

    /// Enable/disable CORS (overrides config file).
    #[arg(long)]
    cors_enabled: Option<bool>,
}

/// Load the effective configuration by merging CLI args over a config file.
///
/// Priority (highest wins): CLI flag > config file > built-in default.
pub fn load_config() -> Result<ServerConfig, String> {
    let cli = CliArgs::parse();

    let mut config = ServerConfig::default();

    if let Some(config_path) = &cli.config {
        let contents = std::fs::read_to_string(config_path)
            .map_err(|e| format!("failed to read config file '{}': {e}", config_path.display()))?;

        let file_config: ServerConfig = if config_path.extension().is_some_and(|e| e == "json") {
            serde_json::from_str(&contents)
                .map_err(|e| format!("failed to parse JSON config: {e}"))?
        } else {
            toml::from_str(&contents)
                .map_err(|e| format!("failed to parse TOML config: {e}"))?
        };

        merge_config(&mut config, file_config);
    }

    if let Some(host) = cli.host {
        config.host = host;
    }
    if let Some(port) = cli.port {
        config.port = port;
    }
    if let Some(path) = cli.data_path {
        config.data_path = path;
    }
    if let Some(level) = cli.log_level {
        config.log_level = level;
    }
    if let Some(key) = cli.api_key {
        config.api_key = Some(key);
    }
    if let Some(enabled) = cli.auth_enabled {
        config.auth_enabled = enabled;
    }
    if let Some(enabled) = cli.rate_limit_enabled {
        config.rate_limit_enabled = enabled;
    }
    if let Some(rpm) = cli.rate_limit_requests_per_minute {
        config.rate_limit_requests_per_minute = rpm;
    }
    if let Some(enabled) = cli.cors_enabled {
        config.cors_enabled = enabled;
    }

    Ok(config)
}

/// Merge a file config into the base config (non-default values win).
fn merge_config(base: &mut ServerConfig, file: ServerConfig) {
    *base = file;
}
