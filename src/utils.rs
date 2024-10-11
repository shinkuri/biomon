use chrono::{TimeZone, Utc};

pub fn format_timestamp(timestamp: i64) -> String {
    Utc.timestamp_opt(timestamp, 0).unwrap().to_rfc3339()
}