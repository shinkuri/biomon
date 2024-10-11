use std::str::SplitWhitespace;

use chrono::Utc;
use rusqlite::{params, Connection};

use crate::Stat;

pub struct Mood;

impl Stat for Mood {
    fn tables(conn: &Connection) {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS mood (
                    id          INTEGER PRIMARY KEY,
                    timestamp   INTEGER NOT NULL,
                    mood        TEXT NOT NULL
                );", 
            [],
        )
        .expect("Failed to ensure table 'mood' exists");
    }

    fn command(input: &mut SplitWhitespace, conn: &Connection) -> String {
        let mood = match input.next() {
            Some(mood) => mood,
            None => return String::from("Failed to extract parameter")
        };
    
        let timestamp = Utc::now().timestamp();
        conn.execute(
            "INSERT INTO mood (timestamp, mood) VALUES (?1, ?2);",
            params![timestamp, mood]
        )
        .expect("Failed to persist bp data");
    
        format!("Recorded mood: {}", mood)
    }

    fn help() -> String {
        String::from("\tmood <mood:str>\n")
    }
}