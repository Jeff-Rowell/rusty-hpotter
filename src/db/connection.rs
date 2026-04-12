use anyhow::Result;
use sqlx::{PgPool, postgres::PgPoolOptions};

#[derive(Debug)]
pub struct DbConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
    pub max_connections: u32,
}

impl DbConfig {
    /// Creates a database connection string using the provided `database`
    /// name.
    ///
    /// # Arguments
    ///
    /// * `database`: the name of the PostgreSQL database to connect to
    pub fn connection_string(&self, db: &str) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.user, self.password, self.host, self.port, db
        )
    }
}

/// Connects to the database using the given `config`. Ensures the database
/// exists, and will attempt to create it if it does not exist.
///
/// # Arguments
///
/// * `config`: the database configuration
///
/// # Examples
///
/// ```ignore
/// use hpotter::db;
///
/// async fn example() -> Result<()> {
///     let db_conf = db::connection::DbConfig {
///         host: String::from("127.0.0.1"),
///         port: 5432,
///         user: String::from("user"),
///         password: String::from("password"),
///         database: String::from("example"),
///         max_connections: 1,
///     };
///
///     let db = db::connection::connect(&db_conf).await?;
///
///     Ok(())
/// }
/// ```
pub async fn connect(config: &DbConfig) -> Result<PgPool> {
    ensure_database_exists(config).await?;

    let url = config.connection_string(&config.database);
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .connect(&url)
        .await?;

    return Ok(pool);
}

/// Ensures the PostgreSQL database exists and creates it if it doesn't.
///
/// # Arguments
///
/// * `config`: the database configuration
async fn ensure_database_exists(config: &DbConfig) -> Result<()> {
    let admin_url = config.connection_string(&config.database);
    let admin_pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .connect(&admin_url)
        .await?;

    let db_name = &config.database;
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_database WHERE datname = $1)",
    )
    .bind(db_name)
    .fetch_one(&admin_pool)
    .await?;

    if !exists {
        sqlx::query(&format!("CREATE DATABASE \"{}\"", db_name))
            .execute(&admin_pool)
            .await?;
    }

    admin_pool.close().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_credential_connection_string() -> () {
        let database = String::from("test_database_postgres");

        let db_cred = DbConfig {
            host: String::from("db_host"),
            port: 54321,
            user: String::from("test_user"),
            password: String::from("test_password"),
            database: database.clone(),
            max_connections: 1,
        };

        let expected = "postgres://test_user:test_password@db_host:54321/test_database_postgres";
        let actual = db_cred.connection_string(&database);
        assert_eq!(actual, expected);
    }
}
