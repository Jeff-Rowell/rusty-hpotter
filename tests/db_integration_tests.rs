use anyhow::Result;
use bollard::Docker;
use hpotter::config::Config;
use hpotter::db::connection::DbConfig;
use hpotter::db::models::{NewConnection, NewCredential, NewPayload};
use hpotter::{
    db,
    db::connection::connect,
    db::{DbCredentials, get_db_container_conf},
    docker,
    docker::{HpotterContainerConfig, delete_container},
};
use sqlx::PgPool;
use std::sync::Arc;
use test_helpers::{
    TEST_DB_CONNECT_CONFIG, TEST_GET_DB_CONF_CONFIG, TEST_MODELS_DB_CONFIG, TEST_NEW_DB_CONFIG,
};

#[cfg(test)]
mod tests {

    use super::*;

    async fn setup() -> Result<PgPool> {
        let docker = Arc::new(Docker::connect_with_socket_defaults().unwrap());
        let config: Config = serde_norway::from_str(TEST_MODELS_DB_CONFIG).unwrap();
        let db = db::new(&config, &docker).await.unwrap();

        Ok(db)
    }

    async fn teardown() -> Result<()> {
        let config: Config = serde_norway::from_str(TEST_MODELS_DB_CONFIG).unwrap();
        let docker = Arc::new(Docker::connect_with_socket_defaults().unwrap());
        let _ = delete_container(&docker, &config.database.name)
            .await
            .unwrap();
        Ok(())
    }

    #[tokio::test]
    async fn test_new_db() -> () {
        let docker = Arc::new(Docker::connect_with_socket_defaults().unwrap());
        let config: Config = serde_norway::from_str(TEST_NEW_DB_CONFIG).unwrap();
        let db = db::new(&config, &docker).await.unwrap();
        let opts = db.options();
        assert_eq!(opts.get_max_connections(), 1);

        let _ = docker::delete_container(&docker, &config.database.name)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_get_db_container_conf() -> () {
        let config: Config = serde_norway::from_str(TEST_GET_DB_CONF_CONFIG).unwrap();
        let docker = Arc::new(Docker::connect_with_socket_defaults().unwrap());
        let db_creds = DbCredentials {
            username: String::from("test-user"),
            password: String::from("test-password"),
        };

        let db_network_name = config.database.network.clone();
        let db_network_id = docker::ensure_db_network(&docker, &db_network_name)
            .await
            .unwrap();

        let db_volume_name = config.database.volume.clone();

        let expected_db_container_conf = HpotterContainerConfig {
            name: config.database.name.clone(),
            image: String::from(&config.database.image),
            host_port: 4567,
            container_port: 5432,
            env: Some(vec![
                String::from(format!("POSTGRES_DB={}", config.database.name.clone())),
                String::from(format!("POSTGRES_USER={}", db_creds.username)),
                String::from(format!("POSTGRES_PASSWORD={}", db_creds.password)),
            ]),
            network_id: Some(db_network_id),
            cmd: None,
            volumes: Some(vec![String::from(db_volume_name)]),
        };

        let db_container_conf = get_db_container_conf(&config, &docker, &db_creds)
            .await
            .unwrap();

        assert_eq!(db_container_conf, expected_db_container_conf);
    }

    #[tokio::test]
    async fn test_db_connect() -> () {
        let docker = Arc::new(Docker::connect_with_socket_defaults().unwrap());
        let config: Config = serde_norway::from_str(TEST_DB_CONNECT_CONFIG).unwrap();

        let db_creds = DbCredentials::from_env();
        let db_container_conf = get_db_container_conf(&config, &docker, &db_creds)
            .await
            .unwrap();

        let db_container_id = docker::ensure_db_container(&docker, &db_container_conf)
            .await
            .unwrap();
        let _ = docker::start_container(&docker, &db_container_id)
            .await
            .unwrap();

        let db_host = docker::get_container_ip(&docker, &db_container_id)
            .await
            .unwrap();

        let db_conf = DbConfig {
            host: db_host,
            user: db_creds.username,
            password: db_creds.password,
            database: config.database.name.clone(),
            max_connections: config.database.max_connections,
        };

        let db = connect(&db_conf).await.unwrap();

        sqlx::query("SELECT 1")
            .execute(&db)
            .await
            .expect("failed to execute query");

        db.close().await;

        let _ = docker::delete_container(&docker, &config.database.name)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_tables_writes() {
        let pool = setup().await.unwrap();

        let connection = NewConnection {
            image: String::from("some_image:latest"),
            src_port: String::from("8080"),
            dest_port: String::from("8080"),
            src_addr: String::from("192.168.0.1"),
            dest_addr: String::from("8.8.8.8"),
            latitude: String::from("some_lat"),
            longitude: String::from("some_long"),
            country: String::from("some_country"),
            region: String::from("some_region"),
            isp: String::from("some_isp"),
            organization: String::from("some_organization"),
            asn: String::from("some_asn"),
            city: String::from("some_city"),
            zip_code: String::from("some_zip_code"),
            state: String::from("some_state"),
        };

        let new_connection = connection.write(&pool).await.unwrap();

        let row = sqlx::query!("SELECT * from connections WHERE id = $1", new_connection.id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(new_connection.id, row.id);

        let credential = NewCredential {
            username: String::from("some_username"),
            password: String::from("some_password"),
            connection_id: new_connection.id,
        };

        let new_credential = credential.write(&pool).await.unwrap();

        let row = sqlx::query!("SELECT * from credentials WHERE id = $1", new_credential.id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(new_credential.id, row.id);

        let payload = NewPayload {
            data: String::from("some_exploit_data"),
            connection_id: new_connection.id,
        };

        let new_payload = payload.write(&pool).await.unwrap();

        let row = sqlx::query!("SELECT * from payloads WHERE id = $1", new_payload.id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(new_payload.id, row.id);

        let _ = teardown().await.unwrap();
    }
}
