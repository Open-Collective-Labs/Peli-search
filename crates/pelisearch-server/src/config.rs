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

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            data_path: default_data_path(),
            log_level: default_log_level(),
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
}

/// Load the effective configuration by merging CLI args over a config file.
///
/// Priority (highest wins): CLI flag > config file > built-in default.
pub fn load_config() -> Result<ServerConfig, String> {
    let cli = CliArgs::parse();

    // Start with defaults
    let mut config = ServerConfig::default();

    // Overlay config file if provided
    if let Some(config_path) = &cli.config {
        let contents = std::fs::read_to_string(config_path)
            .map_err(|e| format!("failed to read config file '{}': {e}", config_path.display()))?;

        let file_config: ServerConfig = if config_path.extension().map_or(false, |e| e == "json") {
            serde_json::from_str(&contents)
                .map_err(|e| format!("failed to parse JSON config: {e}"))?
        } else {
            // Default to TOML parsing
            toml::from_str(&contents)
                .map_err(|e| format!("failed to parse TOML config: {e}"))?
        };

        // Merge file config into defaults (file values override defaults)
        merge_config(&mut config, file_config);
    }

    // CLI flags override everything
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

    Ok(config)
}

/// Merge a file config into the base config (non-default values win).
fn merge_config(base: &mut ServerConfig, file: ServerConfig) {
    // We rely on serde's default so we can detect non-default by comparing.
    // Simpler approach: just use the file values as-is since they were parsed.
    *base = file;
}
