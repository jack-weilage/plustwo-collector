use eyre::{Context, Result};
use sqlx::types::chrono::NaiveDateTime;
use twitch_api::types::Timestamp;

pub fn timestamp_to_time(ts: &Timestamp) -> Result<NaiveDateTime> {
    NaiveDateTime::parse_from_str(ts.as_str(), "%Y-%m-%dT%H:%M:%S%.f%Z")
        .wrap_err("Failed to transform timestamp to datetime")
}
