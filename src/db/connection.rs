use anyhow::Result;
use sqlx::{PgPool, postgres::PgPoolOptions};

#[derive(Debug)]
pub struct DbConfig {
    pub host: String,
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
    ///
    /// # Examples
    ///
    /// ```
    /// use anyhow::Result;
    /// use hpotter::db::connection::DbConfig;
    ///
    /// async fn example() -> Result<()> {
    ///     let database = String::from("example_postgres_db");
    ///     let db_cred = DbConfig {
    ///         host: String::from("example_db_host"),
    ///         user: String::from("example_user"),
    ///         password: String::from("example_password"),
    ///         database: database.clone(),
    ///         max_connections: 1,
    ///     };
    ///     let expected =
    ///     "postgres://example_user:example_password@example_db_host:54321/example_postgres_db";
    ///     let actual  = db_cred.connection_string(&database);
    ///     assert_eq!(actual, expected);
    ///     Ok(())
    /// }
    /// ```
    pub fn connection_string(&self, db: &str) -> String {
        format!(
            "postgres://{}:{}@{}:5432/{}",
            self.user, self.password, self.host, db
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
    let url = config.connection_string(&config.database);
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .connect(&url)
        .await?;

    return Ok(pool);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_credential_connection_string() -> () {
        let database = String::from("test_database_postgres");

        let db_cred = DbConfig {
            host: String::from("db_host"),
            user: String::from("test_user"),
            password: String::from("test_password"),
            database: database.clone(),
            max_connections: 1,
        };

        let expected = "postgres://test_user:test_password@db_host:5432/test_database_postgres";
        let actual = db_cred.connection_string(&database);
        assert_eq!(actual, expected);
    }
}
