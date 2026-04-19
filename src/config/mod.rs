use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub services: Vec<ServiceConfig>,
    pub database: DatabaseConfig,
}

#[derive(Debug, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub num_threads: u32,
    pub listen_address: String,
    pub listen_port: u16,
    pub listen_proto: String,
    pub image: String,
    pub container_port: u16,
    pub username_pattern: String,
    pub password_pattern: String,
    pub payload_pattern: String,
    pub generate_certs: Option<bool>,
    pub env: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub image: String,
    pub port: u16,
    pub max_connections: u32,
    pub name: String,
    pub network: String,
    pub volume: String,
}

/// Deserializes a YAML configuration for the honeypot from the given path.
///
/// # Arguments
///
/// * `path`: the path to the YAML configuration file.
///
/// # Errors
///
/// Returns an error if the file cannot be read of it the YAML is invalid.
///
/// # Examples
///
/// ```ignore
/// use hpotter::config::load_config;
///
/// let config = load_config("config.yml").unwrap_or_else(|err| {
///     eprintln!("failed to load hpotter config: {err}");
///     process::exit(1);
/// });
/// ```
pub fn load_config(path: &str) -> Result<Config> {
    let contents = std::fs::read_to_string(path).context("failed to read yaml configuration")?;
    serde_norway::from_str(&contents).context("failed to deserialize yaml configuration")
}
