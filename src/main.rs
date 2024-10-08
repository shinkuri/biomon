use std::{io, str::SplitWhitespace};
use chrono::Utc;
use rusqlite::{params, Connection, Result};

fn main() {
    let conn = Connection::open("biomon.sqlite")
        .expect("Failed to open biomon.sqlite");
    conn.execute(
        "CREATE TABLE IF NOT EXISTS weight (
                id          INTEGER PRIMARY KEY,
                weight      REAL NOT NULL,
                timestamp   INTEGER NOT NULL
            );", 
        [],
    )
    .expect("Failed to ensure table 'weight' exists");

    println!("Hello Shinkuri!");
    println!("NOTE: Commonly used unit are implied for all entered data.");
    println!("NOTE: Enter 'help' to see help");

    let mut input = String::new();

    // Wait for user input
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read input");

    // Trim the input to remove any newline or spaces
    let mut input = input.split_whitespace();

    let command = match input.next() {
        Some(command) => command,
        None => {
            println!("Failed to extract command");
            return;
        }
    };
    // Match against the input string
    match command {
        "help" => println!("{}", help()),
        "weight" => println!("{}", weight(&mut input, conn)),
        "bp" => println!("{}", bp(&mut input)),
        "mood" => println!("{}", mood(&mut input)),
        _ => println!("Unknown command: {}", command)
    }
}

fn help() -> String {
    String::from("List of commands: weight, bp, mood")
}

struct Weight {
    id: i32,
    weight: f64,
    timestamp: i64
}

fn weight(input: &mut SplitWhitespace, conn: Connection) -> String {
    let weight = match input.next() {
        Some(weight) => weight,
        None => return String::from("Failed to extract parameter")
    };

    let weight: f64 = match weight.parse() {
        Ok(weight) => weight,
        Err(e) => return format!("Failed to parse parameter: {}", e)
    };

    let utc_unix_time = Utc::now().timestamp();
    conn.execute(
        "INSERT INTO weight (weight, timestamp) VALUES (?1, ?2);",
        params![weight, utc_unix_time]
    )
    .expect("Failed to persist weight data");

    format!("Recorded weight: {}kg", weight)
}

fn bp(input: &mut SplitWhitespace) -> String {
    let sys = match input.next() {
        Some(sys) => sys,
        None => return String::from("Failed to extract parameter: sys")
    };
    let dia = match input.next() {
        Some(dia) => dia,
        None => return String::from("Failed to extract parameter: dia")
    };
    format!("Recorded bp: systolic {}mmHg, diastolic {}mmHg", sys, dia)
}

fn mood(input: &mut SplitWhitespace) -> String {
    let mood = match input.next() {
        Some(mood) => mood,
        None => return String::from("Failed to extract parameter")
    };
    format!("Recorded mood: {}", mood)
}
