use anyhow::Result;
use chrono;
use sqlx::{PgPool, types::uuid};

pub struct NewConnection {
    pub image: String,
    pub src_port: String,
    pub dest_port: String,
    pub src_addr: String,
    pub dest_addr: String,
    pub latitude: String,
    pub longitude: String,
    pub country: String,
    pub region: String,
    pub isp: String,
    pub organization: String,
    pub asn: String,
    pub city: String,
    pub zip_code: String,
    pub state: String,
}

pub struct Connection {
    pub id: uuid::Uuid,
    pub image: String,
    pub src_port: String,
    pub dest_port: String,
    pub src_addr: String,
    pub dest_addr: String,
    pub latitude: String,
    pub longitude: String,
    pub country: String,
    pub region: String,
    pub isp: String,
    pub organization: String,
    pub asn: String,
    pub city: String,
    pub zip_code: String,
    pub state: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl NewConnection {
    pub async fn write(&self, pool: &PgPool) -> Result<Connection, sqlx::Error> {
        let connection = sqlx::query_as!(
            Connection,
            r#"
            INSERT INTO connections (
                image,
                src_port,
                dest_port,
                src_addr,
                dest_addr,
                latitude,
                longitude,
                country,
                region,
                isp,
                organization,
                asn,
                city,
                zip_code,
                state
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15
            )
            RETURNING *
            "#,
            self.image,
            self.src_port,
            self.dest_port,
            self.src_addr,
            self.dest_addr,
            self.latitude,
            self.longitude,
            self.country,
            self.region,
            self.isp,
            self.organization,
            self.asn,
            self.city,
            self.zip_code,
            self.state,
        )
        .fetch_one(pool)
        .await?;

        Ok(connection)
    }
}

pub struct NewCredential {
    pub username: String,
    pub password: String,
    pub connection_id: uuid::Uuid,
}

pub struct Credential {
    pub id: uuid::Uuid,
    pub username: String,
    pub password: String,
    pub connection_id: uuid::Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl NewCredential {
    pub async fn write(&self, pool: &PgPool) -> Result<Credential, sqlx::Error> {
        let credentials = sqlx::query_as!(
            Credential,
            r#"
            INSERT INTO credentials
            (
                username,
                password,
                connection_id
            ) VALUES (
                $1, $2, $3
            )
            RETURNING *
            "#,
            self.username,
            self.password,
            self.connection_id
        )
        .fetch_one(pool)
        .await?;

        Ok(credentials)
    }
}

pub struct NewPayload {
    pub data: String,
    pub connection_id: uuid::Uuid,
}

pub struct Payload {
    pub id: uuid::Uuid,
    pub data: String,
    pub connection_id: uuid::Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl NewPayload {
    pub async fn write(&self, pool: &PgPool) -> Result<Payload, sqlx::Error> {
        let payload = sqlx::query_as!(
            Payload,
            r#"
            INSERT INTO payloads
            (
                data,
                connection_id
            ) VALUES (
                $1, $2
            )
            RETURNING *
            "#,
            self.data,
            self.connection_id,
        )
        .fetch_one(pool)
        .await?;

        Ok(payload)
    }
}
