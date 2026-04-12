mod fixtures;
use bollard::Docker;
use fixtures::TEST_DB_CONFIG;
use hpotter::config::Config;
use hpotter::db;
use hpotter::db::connection::DbConfig;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;

    async fn drop_database(config: &DbConfig, db_to_drop: &str) -> () {
        let admin_url = config.connection_string("postgres");
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .connect(&admin_url)
            .await
            .expect("failed to connect to db for cleanup");

        sqlx::query(&format!("DROP DATABASE IF EXISTS \"{}\"", db_to_drop))
            .execute(&pool)
            .await
            .expect("failed to drop test database");
    }

    #[tokio::test]
    async fn test_new_db() -> () {
        let docker = Arc::new(Docker::connect_with_socket_defaults().unwrap());
        let config: Config = serde_norway::from_str(TEST_DB_CONFIG).unwrap();
        let db = db::new(&config, &docker).await.unwrap();
        let opts = db.options();
        assert_eq!(opts.get_max_connections(), 1);
    }

    #[tokio::test]
    async fn test_get_db_container_conf() -> () {
        todo!()
    }

    #[tokio::test]
    async fn test_db_connect() -> () {
        todo!()
    }
}
