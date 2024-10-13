use std::str::SplitWhitespace;

use chrono::Utc;
use rusqlite::{params, Connection};

use crate::{utils, Stat};

struct HeartrateORM {
    _id: i64,
    timestamp: i64,
    heartrate: i16
}

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
        let param = match input.next() {
            Some(param) => param,
            None => return String::from("No further parameters")
        };
    
        match param {
            "last" => last(input, conn),
            _ => {
                let heartrate = match param.parse::<i64>() {
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
        }
    }

    fn help() -> String {
        String::from("\theartrate <last <count:i64> | heartrate:i16>\n")
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
        SELECT id, timestamp, heartrate
        FROM heartrate
        ORDER BY timestamp DESC
        LIMIT (?1);
    ")
    .expect("Failed to prepare query last");
    
    let results = query.query_map([take], |row| {
        Ok(HeartrateORM {
            _id: row.get(0)?,
            timestamp: row.get(1)?,
            heartrate: row.get(2)?
        })
    });
    
    match results {
        Ok(results) => {
            for result in results {
                let result = result.unwrap();
                output.push_str(&format!("{}bpm, recorded {}\n", result.heartrate, utils::format_timestamp(result.timestamp)));
            }
        },
        Err(err) => output.push_str(&format!("Failed to retrieve last {} entries: {}", take, err)),
    }

    output
}