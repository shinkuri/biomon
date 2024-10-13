use std::{fs, io, str::SplitWhitespace};
use bp::BP;
use chrono::{TimeZone, Utc};
use heartrate::Heartrate;
use mood::Mood;
use rusqlite::{params, Connection};
use weight::Weight;

mod weight;
mod bp;
mod mood;
mod heartrate;
mod utils;

pub trait Stat {
    fn tables(conn: &Connection);
    fn command(input: &mut SplitWhitespace, conn: &Connection) -> String;
    fn help() -> String;
}

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
            "weight" => println!("{}", Weight::command(&mut input, &conn)),
            "bp" => println!("{}", BP::command(&mut input, &conn)),
            "mood" => println!("{}", Mood::command(&mut input, &conn)),
            "heartrate" => println!("{}", Heartrate::command(&mut input, &conn)),
            "ingest_markdown_weight" => ingest_markdown_weight(&mut input, &conn),
            "q" => running = false,
            _ => println!("Unknown command: {}", command)
        }
    }

}

fn create_tables(conn: &Connection) {
    Weight::tables(conn);
    BP::tables(conn);
    Mood::tables(conn);
    Heartrate::tables(conn);
}

fn help() -> String {
    let mut help = String::new();
    help.push_str("List of commands:\n");
    help.push_str(&Weight::help());
    help.push_str(&BP::help());
    help.push_str(&Mood::help());
    help.push_str(&Heartrate::help());
    help.push_str("\tingest_markdown_weight <file_path:str>");
    help.push_str("\tq -> exit");
    
    help
}

fn ingest_markdown_weight(input: &mut SplitWhitespace, conn: &Connection) {
    let file = match input.next() {
        Some(file) => file,
        None => return
    };

    let contents = fs::read_to_string(file).unwrap();

    for line in contents.lines() {
        // - 2023-12-03: 105.4kg
        let line = line.replace("- ", "");
        // 2023-12-03: 105.4kg
        let mut parts = line.split(':');
        // ["2023-12-03", " 105.4kg"]
        let date = match parts.next() {
            Some(date) => date,
            None => continue
        };
        let mut date_parts = date.split('-');
        let year = match date_parts.next().and_then(|year| year.parse::<i32>().ok()) {
            Some(year) => year,
            None => continue
        };
        let month = match date_parts.next().and_then(|month| month.parse::<u32>().ok()) {
            Some(month) => month,
            None => continue
        };
        let day = match date_parts.next().and_then(|day| day.parse::<u32>().ok()) {
            Some(day) => day,
            None => continue
        };
        // assume measurements were taken at 09:00
        let timestamp = Utc.with_ymd_and_hms(year, month, day, 9, 0, 0).unwrap().timestamp();
        
        let weight = match parts.next() {
            Some(weight) => weight.trim().replace("kg", ""),
            None => continue
        };

        conn.execute(
            "INSERT INTO weight (timestamp, weight) VALUES (?1, ?2);",
            params![timestamp, weight]
        )
        .expect("Failed to persist weight data");
    
        println!("Recorded weight: {}kg", weight)
    }
}
