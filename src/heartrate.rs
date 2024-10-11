use std::str::SplitWhitespace;

use chrono::Utc;
use rusqlite::{params, Connection};

use crate::Stat;

pub struct Heartrate;

impl Stat for Heartrate {
    fn tables(conn: &Connection) {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS heartrate (
                    id          INTEGER PRIMARY KEY,
                    timestamp   INTEGER NOT NULL,
                    heartrate   INTEGER NOT NULL
                );", 
            [],
        )
        .expect("Failed to ensure table 'heartrate' exists");
    }

    fn command(input: &mut SplitWhitespace, conn: &Connection) -> String {
        let heartrate = match input.next() {
            Some(heartrate) => heartrate,
            None => return String::from("Failed to extract parameter")
        };
    
        let heartrate: i64 = match heartrate.parse() {
            Ok(heartrate) => heartrate,
            Err(e) => return format!("Failed to parse parameter: {}", e)
        };
    
        let timestamp = Utc::now().timestamp();
        conn.execute(
            "INSERT INTO heartrate (timestamp, heartrate) VALUES (?1, ?2);",
            params![timestamp, heartrate]
        )
        .expect("Failed to persist heartrate data");
    
        format!("Recorded heartrate: {}bpm", heartrate)
    }

    fn help() -> String {
        String::from("\theartrate <heartrate:i64>\n")
    }
}