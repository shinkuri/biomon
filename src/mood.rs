use std::str::SplitWhitespace;

use chrono::Utc;
use rusqlite::{params, Connection};

use crate::{utils, Stat};

struct MoodORM {
    _id: i64,
    timestamp: i64,
    mood: String
}

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
        let param = match input.next() {
            Some(param) => param,
            None => return String::from("No further parameters")
        };
        
        match param {
            "last" => last(input, conn),
            _ => {            
                let timestamp = Utc::now().timestamp();
                conn.execute(
                    "INSERT INTO mood (timestamp, mood) VALUES (?1, ?2);",
                    params![timestamp, param]
                )
                .expect("Failed to persist mood data");
            
                format!("Recorded mood: {}", param)
            }
        }
    }

    fn help() -> String {
        String::from("\tmood <last <count:i64> | mood:str>\n")
    }
}

fn last(input: &mut SplitWhitespace, conn: &Connection) -> String {
    let mut output = String::new();
    
    let take_default = 3;
    let take = match input.next() {
        Some(take) => take.parse::<i64>().unwrap_or_else(|_| {
            output.push_str(&format!("Failed to parse query parameter\nUsing default query parameter {}\n", take_default));
            take_default
        }),
        None => {
            output.push_str(&format!("Using default query parameter {}\n", take_default));
            take_default
        }
    };
    
    let mut query = conn.prepare("
        SELECT id, timestamp, mood
        FROM mood
        ORDER BY timestamp DESC
        LIMIT (?1);
    ")
    .expect("Failed to prepare query last");
    
    let results = query.query_map([take], |row| {
        Ok(MoodORM {
            _id: row.get(0)?,
            timestamp: row.get(1)?,
            mood: row.get(2)?
        })
    });
    
    match results {
        Ok(results) => {
            for result in results {
                let result = result.unwrap();
                output.push_str(&format!("{}, recorded {}\n", result.mood, utils::format_timestamp(result.timestamp)));
            }
        },
        Err(err) => output.push_str(&format!("Failed to retrieve last {} entries: {}", take, err)),
    }

    output
}