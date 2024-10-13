use chrono::{Local, TimeZone, Utc};
use chrono::LocalResult::Single;

pub fn format_timestamp(timestamp: i64) -> String {
    match Utc.timestamp_opt(timestamp, 0) {
        Single(dt) => dt.with_timezone(&Local).to_rfc3339(),
        _ => format!("Invalid timestamp {}", timestamp)
    }
}