use config::{Config, ConfigError};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Clone)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub service: ServiceConfig,
    #[allow(unused)]
    pub discord: DiscordConfig,
    pub summary: SummaryConfig,
}

#[derive(Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Deserialize, Clone)]
pub struct ServiceConfig {
    pub produce_digest_interval_seconds: u64,
    pub message_log_directory: PathBuf,
    pub port: u16,
    pub host: String,
    pub max_gpt_request_tokens: usize,
}

#[derive(Deserialize, Clone)]
pub struct DiscordConfig {
    #[allow(unused)]
    pub channel_ids: Vec<String>,
}

#[derive(Deserialize, Clone)]
pub struct SummaryConfig {
    pub model: String,
    pub prompt: String,
    pub max_tokens: usize,
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigError> {
        let file_path = std::env::var("CONFIG_FILE").unwrap_or_else(|_| "config.toml".to_string());
        Self::load_from_file(&file_path)
    }
    pub fn load_from_file(file_path: &str) -> Result<Self, ConfigError> {
        let config = Config::builder()
            .add_source(config::File::with_name(file_path))
            .build()?;

        config.try_deserialize::<Self>()
    }
}
