use std::str::SplitWhitespace;

use chrono::Utc;
use log::error;
use rusqlite::{params, Connection};

use crate::{utils, Stat};

struct WeightORM {
    _id: i64,
    timestamp: i64,
    weight: f64,
}

pub struct Weight;

impl Stat for Weight {
    fn tables(conn: &Connection) {
        let _ = conn
            .execute(
                "CREATE TABLE IF NOT EXISTS weight (
                    id          INTEGER PRIMARY KEY,
                    timestamp   INTEGER UNIQUE NOT NULL,
                    weight      REAL NOT NULL
                );",
                [],
            )
            .map_err(|err| error!("Failed to ensure table 'weight' exists -> {}", err));
    }

    fn command(input: &mut SplitWhitespace, conn: &Connection) -> String {
        let param = match input.next() {
            Some(param) => param,
            None => return String::from("No further parameters"),
        };

        match param {
            "last" => last(input, conn),
            _ => {
                let weight = match param.parse::<f64>() {
                    Ok(weight) => weight,
                    Err(e) => return format!("Failed to parse parameter: {}", e),
                };

                let timestamp = Utc::now().timestamp();
                match conn.execute(
                    "INSERT INTO weight (timestamp, weight) VALUES (?1, ?2);",
                    params![timestamp, weight],
                ) {
                    Ok(_) => format!("Recorded weight: {}kg", weight),
                    Err(err) => {
                        error!("Failed to write weight to database -> {}", err);
                        String::from(
                            "Failed to write weight to database. Check log for full error.",
                        )
                    }
                }
            }
        }
    }

    fn help() -> String {
        String::from("\tweight <last <count:i64> | weight:f64>\n")
    }
}

fn last(input: &mut SplitWhitespace, conn: &Connection) -> String {
    let mut output = String::new();

    let take_default = 3;
    let take = match input.next() {
        Some(take) => take.parse::<i64>().unwrap_or_else(|_| {
            output.push_str(&format!(
                "Failed to parse query parameter\nUsing default query parameter {}\n",
                take_default
            ));
            take_default
        }),
        None => {
            output.push_str(&format!("Using default query parameter {}\n", take_default));
            take_default
        }
    };

    let mut query = match conn.prepare(
        "
        SELECT id, timestamp, weight
        FROM weight
        ORDER BY timestamp DESC
        LIMIT (?1);
    ",
    ) {
        Ok(query) => query,
        Err(err) => {
            error!("Failed to prepare query 'last' for weight -> {}", err);
            output.push_str("Failed to prepare query 'last' for weight. Check log for full error.");
            return output;
        }
    };

    let results = query.query_map([take], |row| {
        Ok(WeightORM {
            _id: row.get(0)?,
            timestamp: row.get(1)?,
            weight: row.get(2)?,
        })
    });

    match results {
        Ok(results) => {
            for result in results {
                let result = result.unwrap();
                output.push_str(&format!(
                    "{}kg, recorded {}\n",
                    result.weight,
                    utils::format_timestamp(result.timestamp)
                ));
            }
        }
        Err(err) => output.push_str(&format!(
            "Failed to retrieve last {} entries: {}\n",
            take, err
        )),
    }

    output
}
