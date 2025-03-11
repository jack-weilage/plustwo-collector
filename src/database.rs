use eyre::{Context, Result};
use sqlx::{PgPool, postgres::PgPoolOptions};
use twitch_api::types::{Timestamp, UserId};

use crate::time::timestamp_to_time;

pub struct DatabaseClient {
    pool: PgPool,
}
impl DatabaseClient {
    pub async fn new(url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await
            .wrap_err("Failed to connect to database")?;

        Ok(Self { pool })
    }
    #[tracing::instrument(skip(self))]
    pub async fn insert_message(
        &self,
        timestamp: Timestamp,
        user_id: UserId,
        channel_id: UserId,
        kind: MessageKind,
    ) -> Result<()> {
        tracing::info!("inserting new message into database");

        sqlx::query(
            r#"
            INSERT INTO messages (timestamp, user_id, channel_id, message_kind)
                VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(timestamp_to_time(&timestamp)?)
        .bind(user_id.as_str().parse::<i64>()?)
        .bind(channel_id.as_str().parse::<i64>()?)
        .bind(kind)
        .execute(&self.pool)
        .await
        .wrap_err("Failed to insert message")?;

        Ok(())
    }
    pub async fn insert_event(
        &self,
        timestamp: Timestamp,
        broadcaster_id: UserId,
        kind: EventKind,
    ) -> Result<()> {
        tracing::info!("inserting new event into database");

        sqlx::query(
            r#"
            INSERT INTO events (timestamp, broadcaster_id, event_kind)
                VALUES ($1, $2, $3)
            "#,
        )
        .bind(timestamp_to_time(&timestamp)?)
        .bind(broadcaster_id.as_str().parse::<i64>()?)
        .bind(kind)
        .execute(&self.pool)
        .await
        .wrap_err("Failed to insert message")?;

        Ok(())
    }
}

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "message_kind", rename_all = "kebab-case")]
pub enum MessageKind {
    PlusTwo,
    MinusTwo,
}

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "event_kind", rename_all = "kebab-case")]
pub enum EventKind {
    BroadcastOnline,
    BroadcastOffline,
}
