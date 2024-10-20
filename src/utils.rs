use std::sync::{Arc, RwLock};

use chrono::LocalResult::Single;
use chrono::{Local, TimeZone, Utc};
use log::error;

use crate::SectionedConfigMap;

pub fn format_timestamp(timestamp: i64) -> String {
    match Utc.timestamp_opt(timestamp, 0) {
        Single(dt) => dt.with_timezone(&Local).to_rfc3339(),
        _ => format!("Invalid timestamp {}", timestamp),
    }
}

pub fn from_config(
    conf: Arc<RwLock<SectionedConfigMap>>,
    section: &str,
    key: &str,
) -> Result<String, ()> {
    let conf_guard = match conf.read() {
        Ok(conf_guard) => conf_guard,
        Err(err) => {
            error!("Failed to aquire lock on config map -> {}", err);
            return Err(());
        }
    };

    conf_guard
        .get(section)
        .ok_or_else(|| {
            error!("Missing config section {}", section);
        })
        .and_then(|secmap| {
            secmap.get(key).ok_or_else(|| {
                error!("Missing config key {} in section {}", key, section);
            })
        })
        .and_then(|kv| {
            kv.as_ref().ok_or_else(|| {
                error!("Missing config value for {} in section {}", key, section);
            })
        })
        .cloned()
}
