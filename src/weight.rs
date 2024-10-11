use std::str::SplitWhitespace;

use chrono::Utc;
use rusqlite::{params, Connection};

use crate::{utils, Stat};

struct WeightORM {
    _id: i32,
    timestamp: i64,
    weight: f64
}

pub struct Weight;

impl Stat for Weight {
    fn tables(conn: &Connection) {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS weight (
                    id          INTEGER PRIMARY KEY,
                    timestamp   INTEGER NOT NULL,
                    weight      REAL NOT NULL
                );", 
            [],
        )
        .expect("Failed to ensure table 'weight' exists");
    }

    fn command(input: &mut SplitWhitespace, conn: &Connection) -> String {
        let weight = match input.next() {
            Some(weight) => weight,
            None => return String::from("Failed to extract parameter")
        };
    
        let weight = match weight.parse::<f64>() {
            Ok(weight) => weight,
            Err(e) => return format!("Failed to parse parameter: {}", e)
        };
    
        let timestamp = Utc::now().timestamp();
        conn.execute(
            "INSERT INTO weight (timestamp, weight) VALUES (?1, ?2);",
            params![timestamp, weight]
        )
        .expect("Failed to persist weight data");
    
        format!("Recorded weight: {}kg", weight)
    }

    fn help() -> String {
        String::from("\tweight <weight:f64>\n")
    }
}

pub fn stat_weight(input: &mut SplitWhitespace, conn: &Connection) -> String {
    let mut output = String::new();

    let query_command = match input.next() {
        Some(query_command) => query_command,
        None => return String::from("Failed to extract query command")
    };

    let query_param = input.next();
    
    match query_command {
        "last" => {
            let take_default = 10;
            let take = match query_param {
                Some(take) => take.parse::<i32>().unwrap_or_else(|_| {
                    output.push_str("Failed to parse query parameter\nUsing default query parameter\n");
                    take_default
                }),
                None => {
                    output.push_str("Using default query parameter\n");
                    take_default
                }
            };
            
            let mut query = conn.prepare("
                SELECT id, timestamp, weight
                FROM weight
                ORDER BY timestamp DESC
                LIMIT (?1);
            ")
            .expect("Failed to prepare query last");
            
            let results = query.query_map([take], |row| {
                Ok(WeightORM {
                    _id: row.get(0)?,
                    timestamp: row.get(1)?,
                    weight: row.get(2)?
                })
            });
            
            match results {
                Ok(results) => {
                    let mut records = String::new();
                    for result in results {
                        let result = result.unwrap();
                        records.push_str(&format!("{}kg, recorded {}\n", result.weight, utils::format_timestamp(result.timestamp)));
                    }
                    records
                },
                Err(err) => format!("Failed to retrieve last {} weights: {}", take, err)
            }
        },
        _ => format!("Unknown query command: {}", query_command)
    }
}