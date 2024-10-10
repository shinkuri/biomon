use std::{io, str::SplitWhitespace};
use chrono::{TimeZone, Utc};
use rusqlite::{params, Connection};

fn main() {
    let conn = Connection::open("biomon.sqlite")
        .expect("Failed to open biomon.sqlite");
    create_tables(&conn);

    println!("Hello Shinkuri!");
    println!("NOTE: Commonly used unit are implied for all entered data.");
    println!("NOTE: Enter 'help' to see help");

    let mut running = true;

    while running {
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
            "weight" => println!("{}", weight(&mut input, &conn)),
            "bp" => println!("{}", bp(&mut input, &conn)),
            "mood" => println!("{}", mood(&mut input, &conn)),
            "stat" => println!("{}", stat(&mut input, &conn)),
            "q" => running = false,
            _ => println!("Unknown command: {}", command)
        }
    }

}

fn help() -> String {
    String::from("List of commands: \n
        \tweight <weight:f64>\n
        \tbp <sys:i64> <dia:i64>\n
        \tmood <mood:str>\n
        \tstat <command:str> <query:str>\n
        \tq -> exit
    ")
}

struct Weight {
    id: i32,
    timestamp: i64,
    weight: f64
}

fn weight(input: &mut SplitWhitespace, conn: &Connection) -> String {
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
        "INSERT INTO weight (timestamp, weight) VALUES (?1, ?2);",
        params![utc_unix_time, weight]
    )
    .expect("Failed to persist weight data");

    format!("Recorded weight: {}kg", weight)
}

fn bp(input: &mut SplitWhitespace, conn: &Connection) -> String {
    let sys = match input.next() {
        Some(sys) => sys,
        None => return String::from("Failed to extract parameter: sys")
    };
    let dia = match input.next() {
        Some(dia) => dia,
        None => return String::from("Failed to extract parameter: dia")
    };

    let sys: i64 = match sys.parse() {
        Ok(sys) => sys,
        Err(e) => return format!("Failed to parse parameter: {}", e)
    };
    let dia: i64 = match dia.parse() {
        Ok(dia) => dia,
        Err(e) => return format!("Failed to parse parameter: {}", e)
    };

    let utc_unix_time = Utc::now().timestamp();
    conn.execute(
        "INSERT INTO bp (timestamp, sys, dia) VALUES (?1, ?2, ?3);",
        params![utc_unix_time, sys, dia]
    )
    .expect("Failed to persist bp data");

    format!("Recorded bp: systolic {}mmHg, diastolic {}mmHg", sys, dia)
}

fn mood(input: &mut SplitWhitespace, conn: &Connection) -> String {
    let mood = match input.next() {
        Some(mood) => mood,
        None => return String::from("Failed to extract parameter")
    };

    let utc_unix_time = Utc::now().timestamp();
    conn.execute(
        "INSERT INTO mood (timestamp, mood) VALUES (?1, ?2);",
        params![utc_unix_time, mood]
    )
    .expect("Failed to persist bp data");

    format!("Recorded mood: {}", mood)
}

fn stat(input: &mut SplitWhitespace, conn: &Connection) -> String {
    let sub_command = match input.next() {
        Some(sub_command) => sub_command,
        None => return String::from("Failed to extract sub command")
    };

    match sub_command {
        "weight" => stat_weight(input, conn),
        _ => format!("Unknown stat command: {}", sub_command)
    }
}

fn stat_weight(input: &mut SplitWhitespace, conn: &Connection) -> String {
    let query_command = match input.next() {
        Some(query_command) => query_command,
        None => return String::from("Failed to extract query command")
    };
    
    match query_command {
        "last" => {
            let take = 10;
            
            let mut query = conn.prepare("
                SELECT id, timestamp, weight
                FROM weight
                ORDER BY timestamp DESC
                LIMIT (?1);
            ")
            .expect("Failed to prepare query last");
            
            let results = query.query_map([take], |row| {
                Ok(Weight {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    weight: row.get(2)?
                })
            });
            
            match results {
                Ok(results) => {
                    let mut records = String::new();
                    for result in results {
                        let result = result.unwrap();
                        records.push_str(&format!("{}kg, recorded {}\n", result.weight, format_timestamp(result.timestamp)));
                    }
                    records
                },
                Err(err) => format!("Failed to retrieve last {} weights: {}", take, err)
            }
        },
        _ => format!("Unknown query command: {}", query_command)
    }
}

fn create_tables(conn: &Connection) {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS weight (
                id          INTEGER PRIMARY KEY,
                timestamp   INTEGER NOT NULL,
                weight      REAL NOT NULL
            );", 
        [],
    )
    .expect("Failed to ensure table 'weight' exists");
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

fn format_timestamp(timestamp: i64) -> String {
    Utc.timestamp_opt(timestamp, 0).unwrap().to_rfc3339()
}
