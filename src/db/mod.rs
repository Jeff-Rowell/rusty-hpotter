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
