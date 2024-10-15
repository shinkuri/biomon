use std::str::SplitWhitespace;
use std::{error::Error, fmt::Write};

use chrono::Utc;
use log::error;
use rusqlite::{params, Connection};

use crate::{utils, Stat};

struct HeartrateORM {
    _id: i64,
    timestamp: i64,
    heartrate: u8,
    duration: i16,
}

pub struct Heartrate;

impl Stat for Heartrate {
    fn tables(conn: &Connection) {
        let _ = conn
            .execute(
                "CREATE TABLE IF NOT EXISTS heartrate (
                    id          INTEGER PRIMARY KEY,
                    timestamp   INTEGER UNIQUE NOT NULL,
                    heartrate   INTEGER NOT NULL,
                    duration    INTEGER DEFAULT (0) NOT NULL
                );",
                [],
            )
            .map_err(|err| error!("Failed to ensure table 'heartrate' exists -> {}", err));
    }

    fn command(input: &mut SplitWhitespace, conn: &Connection) -> String {
        let param = match input.next() {
            Some(param) => param,
            None => return String::from("No further parameters"),
        };

        match param {
            "last" => last(input, conn),
            "compress" => {
                let mut query = conn
                    .prepare(
                        "
                    SELECT id, timestamp, heartrate, duration
                    FROM heartrate
                    ORDER BY timestamp DESC;
                ",
                    )
                    .expect("Failed to prepare query last");

                let results = query.query_map([], |row| {
                    Ok(HeartrateORM {
                        _id: row.get(0)?,
                        timestamp: row.get(1)?,
                        heartrate: row.get(2)?,
                        duration: row.get(3)?,
                    })
                });

                match results {
                    Ok(results) => {
                        // Read from db into memory
                        let mut raw = Vec::<HeartrateORM>::new();

                        for result in results {
                            raw.push(result.unwrap());
                        }

                        // Compress
                        let compressed = rle_encode(raw);

                        // Write from memory into db
                        let mut update = String::new();
                        for hr in compressed {
                            let _ = writeln!(
                                update,
                                "UPDATE heartrate SET timestamp = {}, heartrate = {}, duration = {} WHERE id = {};",
                                hr.timestamp, hr.heartrate, hr.duration, hr._id
                            );
                        }
                        if let Err(err) = conn.execute_batch(&update).map_err(|err| {
                            error!("Failed to persist compression -> {}", err);
                            String::from("Failed to persist compression. Check log for full error.")
                        }) {
                            return err;
                        }

                        // Delete all entries with duration = -1
                        match conn.execute("DELETE FROM heartrate WHERE duration = -1;", []) {
                            Ok(deleted) => format!("Reduced entries by {}", deleted),
                            Err(err) => {
                                error!("Failed to clean up heartrate data -> {}", err);
                                String::from(
                                    "Failed to clean up heartrate data. Check log for full error.",
                                )
                            }
                        }
                    }
                    Err(err) => format!("Failed to read heartrate history\n{}", err),
                }
            }
            _ => {
                let heartrate = match param.parse::<u8>() {
                    Ok(heartrate) => heartrate,
                    Err(e) => return format!("Failed to parse parameter: {}", e),
                };

                match write_heartrate(heartrate, conn) {
                    Ok(_) => format!("Recorded heartrate: {}bpm", heartrate),
                    Err(err) => format!("Failed to write heartrate data\n{}", err),
                }
            }
        }
    }

    fn help() -> String {
        String::from("\theartrate <last <count:i64> | heartrate:u8>\n")
    }
}

pub fn write_heartrate(value: u8, conn: &Connection) -> Result<usize, Box<dyn Error>> {
    let timestamp = Utc::now().timestamp();
    Ok(conn.execute(
        "INSERT INTO heartrate (timestamp, heartrate) VALUES (?1, ?2);",
        params![timestamp, value],
    )?)
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

    let mut query = conn
        .prepare(
            "
        SELECT id, timestamp, heartrate, duration
        FROM heartrate
        ORDER BY timestamp DESC
        LIMIT (?1);
    ",
        )
        .expect("Failed to prepare query last");

    let results = query.query_map([take], |row| {
        Ok(HeartrateORM {
            _id: row.get(0)?,
            timestamp: row.get(1)?,
            heartrate: row.get(2)?,
            duration: row.get(3)?,
        })
    });

    match results {
        Ok(results) => {
            for result in results {
                let result = result.unwrap();
                output.push_str(&format!(
                    "{}bpm ({}s), recorded {}\n",
                    result.heartrate,
                    result.duration,
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

// marks elements that should not be persistet with duration = -1
fn rle_encode(mut raw: Vec<HeartrateORM>) -> Vec<HeartrateORM> {
    if raw.len() < 2 {
        return raw;
    };

    println!("Compressing {} elements", raw.len());

    let step: i16 = 1; // readings happen every second, ideally

    let mut iter = raw.iter_mut();

    let mut this = iter.next().unwrap();
    let mut next = iter.next().unwrap();
    let mut c = 1;
    loop {
        // this.timestamp - next.timestamp == step.into() &&
        if this.heartrate == next.heartrate {
            this.duration = -1;
            c += 1;
        } else {
            this.duration = step * c;
            c = 1;
        }

        this = next;
        next = match iter.next() {
            Some(next) => next,
            None => {
                next.duration = step * c;
                break;
            }
        }
    }

    raw
}
