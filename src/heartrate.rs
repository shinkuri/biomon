use std::fmt::Write;
use std::str::SplitWhitespace;

use chrono::Utc;
use rusqlite::{params, Connection};

use crate::{utils, Stat};

struct HeartrateORM {
    _id: i64,
    timestamp: i64,
    heartrate: i16,
    duration: i16,
}

pub struct Heartrate;

impl Stat for Heartrate {
    fn tables(conn: &Connection) {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS heartrate (
                    id          INTEGER PRIMARY KEY,
                    timestamp   INTEGER NOT NULL,
                    heartrate   INTEGER NOT NULL,
                    duration    INTEGER DEFAULT (0) NOT NULL
                );",
            [],
        )
        .expect("Failed to ensure table 'heartrate' exists");
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
                        conn.execute_batch(&update)
                            .expect("Failed to persist compression");

                        // Delete all entries with duration = -1
                        let deleted = conn
                            .execute("DELETE FROM heartrate WHERE duration = -1;", [])
                            .expect("Failed to clean up heartrate data");

                        format!("Reduced entries by {}", deleted)
                    }
                    Err(err) => format!("Failed to read heartrate history\n{}", err),
                }
            }
            _ => {
                let heartrate = match param.parse::<i64>() {
                    Ok(heartrate) => heartrate,
                    Err(e) => return format!("Failed to parse parameter: {}", e),
                };

                let timestamp = Utc::now().timestamp();
                conn.execute(
                    "INSERT INTO heartrate (timestamp, heartrate) VALUES (?1, ?2);",
                    params![timestamp, heartrate],
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

    let step: i16 = 1; // how many unix time thingies are between two sensor reads?

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
