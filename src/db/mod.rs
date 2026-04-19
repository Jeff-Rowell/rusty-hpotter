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
    pub fn from_env() -> Self {
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
/// ```ignore
/// async fn example() -> Result<()> {
///    let args = Args::parse();
///    let config = config::load_config(&args.config)?;
///    let docker = Arc::new(docker::connect()?);
///    let db = db::new(&config, &docker).await?;
/// }
/// ```
pub async fn new(hpotter_config: &Config, docker_client: &Docker) -> Result<PgPool> {
    let db_creds = DbCredentials::from_env();
    let db_container_conf = get_db_container_conf(hpotter_config, docker_client, &db_creds).await?;

    let db_container_id = docker::ensure_db_container(&docker_client, &db_container_conf).await?;
    let _ = docker::start_container(&docker_client, &db_container_id).await?;

    let db_host = docker::get_container_ip(&docker_client, &db_container_id).await?;

    let db_conf = DbConfig {
        host: db_host,
        user: db_creds.username,
        password: db_creds.password,
        database: hpotter_config.database.name.clone(),
        max_connections: hpotter_config.database.max_connections,
    };

    let db = connect(&db_conf).await?;

    sqlx::migrate!("./migrations").run(&db).await?;

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
/// ```ignore
/// use hpotter::db;
///
/// async fn example() -> Result<()> {
///     let config = load_config("example-config.yml").unwrap_or_else(|err| {
///         eprintln!("failed to load hpotter config: {err}");
///         process::exit(1);
///     });
///     let docker = Arc::new(Docker::connect_with_socket_defaults().unwrap());
///     let db_creds = DbCredentials {
///         username: String::from("example-user"),
///         password: String::from("example-password"),
///     };
///
///     let db_container_conf = get_db_container_conf(&config, &docker, &db_creds)
///         .await
///         .unwrap();
///     Ok(())
/// }
/// ```
pub async fn get_db_container_conf(
    hpotter_config: &Config,
    docker_client: &Docker,
    db_creds: &DbCredentials,
) -> Result<HpotterContainerConfig> {
    let db_network_name = hpotter_config.database.network.clone();
    let db_network_id = docker::ensure_db_network(&docker_client, &db_network_name).await?;

    let db_volume_name = &hpotter_config.database.volume.clone();
    let _ = docker::ensure_db_volume(&docker_client, &db_volume_name).await?;

    Ok(HpotterContainerConfig {
        name: hpotter_config.database.name.clone(),
        image: hpotter_config.database.image.clone(),
        host_port: hpotter_config.database.port,
        container_port: 5432,
        env: Some(vec![
            String::from(format!(
                "POSTGRES_DB={}",
                hpotter_config.database.name.clone()
            )),
            String::from(format!("POSTGRES_USER={}", db_creds.username)),
            String::from(format!("POSTGRES_PASSWORD={}", db_creds.password)),
        ]),
        network_id: Some(db_network_id),
        cmd: None,
        volumes: Some(vec![String::from(db_volume_name)]),
    })
}
