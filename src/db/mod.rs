pub mod connection;
pub mod models;

use anyhow::Result;
use bollard::Docker;
use sqlx::PgPool;

use crate::{
    config::Config,
    db::connection::{DbConfig, connect},
    docker::{self, HpotterContainerConfig},
};

const DB_NAME: &str = "hpotter";

#[derive(Debug)]
/// A set of database credentials
pub struct DbCredentials {
    /// The username to connect to the database
    pub username: String,
    /// The password to connect to the database
    pub password: String,
}

impl DbCredentials {
    /// Loads database credentials from HPOTTER_DB_USER and
    /// HPOTTER_DB_PASSWORD environment variables.
    fn from_env() -> Self {
        Self {
            username: std::env::var("HPOTTER_DB_USER")
                .expect("HPOTTER_DB_USER environment variable required"),
            password: std::env::var("HPOTTER_DB_PASSWORD")
                .expect("HPOTTER_DB_PASSWORD environment variable required"),
        }
    }
}

/// Creates a new PostgreSQL pool based on the honeypot configuration.
///
/// # Arugments
///
/// * `hpotter_config`: the desiralized YAML honey pot configuration
/// * `docker_client`: the docker server client
///
/// # Examples
///
/// ```
/// ```
pub async fn new(hpotter_config: &Config, docker_client: &Docker) -> Result<PgPool> {
    let db_creds = DbCredentials::from_env();
    let db_container_conf = get_db_container_conf(hpotter_config, docker_client, &db_creds).await?;

    let db_container_id = docker::ensure_db_container(&docker_client, &db_container_conf).await?;
    let _ = docker::start_container(&docker_client, &db_container_id).await?;

    let db_host = docker::get_container_ip(&docker_client, &db_container_id).await?;

    let db_conf = DbConfig {
        host: db_host,
        port: hpotter_config.database.port,
        user: db_creds.username,
        password: db_creds.password,
        database: String::from(DB_NAME),
        max_connections: hpotter_config.database.max_connections,
    };

    let db = connect(&db_conf).await?;

    // TODO: run migrations here?

    Ok(db)
}

/// Builds a `HpotterContainerConfig` for the database container.
///
/// # Arguments
///
/// * `hpotter_config`: the desiralized YAML honey pot configuration
/// * `docker_client`: the docker server client
/// * `db_creds`: the `DbCredentials` object
///
/// # Examples
///
/// ```
/// ```
pub async fn get_db_container_conf(
    hpotter_config: &Config,
    docker_client: &Docker,
    db_creds: &DbCredentials,
) -> Result<HpotterContainerConfig> {
    let db_network_name = "hpotter-db-net";
    let db_network_id = docker::ensure_db_network(&docker_client, db_network_name).await?;

    let db_volume_name = "hpotter-db-data";
    let _ = docker::ensure_db_volume(&docker_client, db_volume_name).await?;

    Ok(HpotterContainerConfig {
        name: String::from(DB_NAME),
        image: hpotter_config.database.image.clone(),
        host_port: hpotter_config.database.port,
        container_port: hpotter_config.database.port,
        env: Some(vec![
            String::from("POSTGRES_DB=hpotter"),
            String::from(format!("POSTGRES_USER={}", db_creds.username)),
            String::from(format!("POSTGRES_PASSWORD={}", db_creds.password)),
        ]),
        network_id: Some(db_network_id),
        cmd: None,
        volumes: Some(vec![String::from(db_volume_name)]),
    })
}
