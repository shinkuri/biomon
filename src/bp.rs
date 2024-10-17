use std::str::SplitWhitespace;

use chrono::Utc;
use log::error;
use rusqlite::{params, Connection};

use crate::{utils, Stat};

struct BloodpressureORM {
    _id: i64,
    timestamp: i64,
    sys: i16,
    dia: i16,
}

pub struct BP;

impl Stat for BP {
    fn tables(conn: &Connection) {
        let _ = conn
            .execute(
                "CREATE TABLE IF NOT EXISTS bp (
                    id          INTEGER PRIMARY KEY,
                    timestamp   INTEGER UNIQUE NOT NULL,
                    sys         INTEGER NOT NULL,
                    dia         INTEGER NOT NULL
                );",
                [],
            )
            .map_err(|err| error!("Failed to ensure table 'bp' exists -> {}", err));
    }

    fn command(input: &mut SplitWhitespace, conn: &Connection) -> String {
        let param = match input.next() {
            Some(param) => param,
            None => return String::from("No further parameters"),
        };

        match param {
            "last" => last(input, conn),
            _ => {
                let sys = match param.parse::<i64>() {
                    Ok(sys) => sys,
                    Err(e) => return format!("Failed to parse parameter: <sys:i16>\n{}", e),
                };

                let dia = match input.next() {
                    Some(dia) => dia,
                    None => return String::from("Missing parameter: dia"),
                };
                let dia = match dia.parse::<i64>() {
                    Ok(dia) => dia,
                    Err(e) => return format!("Failed to parse parameter: <dia:i16>\n{}", e),
                };

                let timestamp = Utc::now().timestamp();
                match conn.execute(
                    "INSERT INTO bp (timestamp, sys, dia) VALUES (?1, ?2, ?3);",
                    params![timestamp, sys, dia],
                ) {
                    Ok(_) => format!("Recorded bp: {}mmHg systolic, {}mmHg diastolic", sys, dia),
                    Err(err) => {
                        error!("Failed to write bp to database -> {}", err);
                        String::from("Failed to write bp to database. Check log for full error.")
                    }
                }
            }
        }
    }

    fn help() -> String {
        String::from("\tbp <last <count:i64>> | <<sys:i16> <dia:i16>>\n")
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
        SELECT id, timestamp, sys, dia
        FROM bp
        ORDER BY timestamp DESC
        LIMIT (?1);
    ",
    ) {
        Ok(query) => query,
        Err(err) => {
            error!("Failed to prepare query 'last' for bp -> {}", err);
            output.push_str("Failed to prepare query 'last' for bp. Check log for full error.");
            return output;
        }
    };

    let results = query.query_map([take], |row| {
        Ok(BloodpressureORM {
            _id: row.get(0)?,
            timestamp: row.get(1)?,
            sys: row.get(2)?,
            dia: row.get(3)?,
        })
    });

    match results {
        Ok(results) => {
            for result in results {
                let result = result.unwrap();
                output.push_str(&format!(
                    "{}mmHg systolic, {}mmHg diastolic, recorded {}\n",
                    result.sys,
                    result.dia,
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
