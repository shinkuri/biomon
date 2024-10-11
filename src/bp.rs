use std::str::SplitWhitespace;

use chrono::Utc;
use rusqlite::{params, Connection};

use crate::Stat;

pub struct BP;

impl Stat for BP {
    fn tables(conn: &Connection) {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS bp (
                    id          INTEGER PRIMARY KEY,
                    timestamp   INTEGER NOT NULL,
                    sys         INTEGER NOT NULL,
                    dia         INTEGER NOT NULL
                );", 
            [],
        )
        .expect("Failed to ensure table 'bp' exists");
    }

    fn command(input: &mut SplitWhitespace, conn: &Connection) -> String {
        let sys = match input.next() {
            Some(sys) => sys,
            None => return String::from("Failed to extract parameter: sys")
        };
        let dia = match input.next() {
            Some(dia) => dia,
            None => return String::from("Failed to extract parameter: dia")
        };
    
        let sys = match sys.parse::<i64>() {
            Ok(sys) => sys,
            Err(e) => return format!("Failed to parse parameter: {}", e)
        };
        let dia = match dia.parse::<i64>() {
            Ok(dia) => dia,
            Err(e) => return format!("Failed to parse parameter: {}", e)
        };
    
        let timestamp = Utc::now().timestamp();
        conn.execute(
            "INSERT INTO bp (timestamp, sys, dia) VALUES (?1, ?2, ?3);",
            params![timestamp, sys, dia]
        )
        .expect("Failed to persist bp data");
    
        format!("Recorded bp: systolic {}mmHg, diastolic {}mmHg", sys, dia)
    }

    fn help() -> String {
        String::from("\tbp <sys:i64> <dia:i64>\n")
    }
}