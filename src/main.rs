use bp::BP;
use chrono::{TimeZone, Utc};
use fern::Dispatch;
use heartrate::Heartrate;
use std::{fs, io, str::SplitWhitespace, time::Duration};

use log::{error, info};
use mood::Mood;
use rusqlite::{backup::Backup, params, Connection};
use weight::Weight;

mod ble_hrp;
mod bp;
mod heartrate;
mod mood;
mod utils;
mod weight;

pub trait Stat {
    fn tables(conn: &Connection);
    fn command(input: &mut SplitWhitespace, conn: &Connection) -> String;
    fn help() -> String;
}

#[tokio::main]
async fn main() {
    setup_logger().expect("Failed to setup logger");
    info!("Biomon launched");

    let conn = match Connection::open("biomon.sqlite") {
        Ok(conn) => conn,
        Err(_) => {
            println!("Failed to open ./biomon.sqlite (Missing permissions?)");
            println!("Cannot proceed without database");
            println!("'q' to exit");

            let mut input = String::new();
            while !input.starts_with('q') {
                // Wait for user input
                let _ = io::stdin().read_line(&mut input).map_err(|err| {
                    error!("Failed to read stdin -> {}", err);
                    println!("Failed to read stdin. Check log for full error.")
                });
            }

            return;
        }
    };

    create_tables(&conn);

    println!("NOTE: Commonly used unit are implied for all entered data");
    println!("NOTE: Enter 'help' to see help");

    let mut running = true;

    while running {
        let mut input = String::new();

        // Wait for user input
        let _ = io::stdin().read_line(&mut input).map_err(|err| {
            error!("Failed to read stdin -> {}", err);
            println!("Failed to read stdin. Check log for full error.")
        });

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
            "record_hrp" => ble_hrp::record_hrp_device("C2:7A:75:27:F7:3E", &conn).await,
            "ingest_markdown_weight" => ingest_markdown_weight(&mut input, &conn),
            "backup" => println!("{}", backup(&mut input, &conn)),
            "q" => running = false,
            _ => println!("Unknown command: {}", command),
        }
    }
}

fn setup_logger() -> Result<(), Box<dyn std::error::Error>> {
    Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                Utc::now().format("[%Y-%m-%d %H:%M:%S]"),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Info) // Set default level
        .chain(std::io::stdout()) // Output to stdout
        .chain(fern::log_file("biomon.log")?)
        .apply()?;
    Ok(())
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
    help.push_str("\trecord_hrp - Connects to BLE HRP compatible device and collects heartrate data");
    help.push_str("\tingest_markdown_weight <file_path:str>\n");
    help.push_str("\tbackup <backup_path:str>");
    help.push_str("\tq -> exit");

    help
}

fn backup(input: &mut SplitWhitespace, conn: &Connection) -> String {
    let path = match input.next() {
        Some(param) => param,
        None => return String::from("Missing destination path"),
    };

    let mut backup_conn = match Connection::open(path) {
        Ok(backup_conn) => backup_conn,
        Err(e) => return format!("Failed to create/open backup target\n{}", e),
    };

    let backup = match Backup::new(conn, &mut backup_conn) {
        Ok(backup) => backup,
        Err(e) => return format!("Failed to initialize backup\n{}", e),
    };

    match backup.run_to_completion(5, Duration::from_millis(250), None) {
        Ok(_) => String::from("backup done"),
        Err(_) => String::from("backup failed"),
    }
}

fn ingest_markdown_weight(input: &mut SplitWhitespace, conn: &Connection) {
    let file = match input.next() {
        Some(file) => file,
        None => return,
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
            None => continue,
        };
        let mut date_parts = date.split('-');
        let year = match date_parts.next().and_then(|year| year.parse::<i32>().ok()) {
            Some(year) => year,
            None => continue,
        };
        let month = match date_parts
            .next()
            .and_then(|month| month.parse::<u32>().ok())
        {
            Some(month) => month,
            None => continue,
        };
        let day = match date_parts.next().and_then(|day| day.parse::<u32>().ok()) {
            Some(day) => day,
            None => continue,
        };
        // assume measurements were taken at 09:00
        let timestamp = Utc
            .with_ymd_and_hms(year, month, day, 9, 0, 0)
            .unwrap()
            .timestamp();

        let weight = match parts.next() {
            Some(weight) => weight.trim().replace("kg", ""),
            None => continue,
        };

        match conn.execute(
            "INSERT INTO weight (timestamp, weight) VALUES (?1, ?2);",
            params![timestamp, weight],
        ) {
            Ok(_) => println!("Recorded weight: {}kg", weight),
            Err(err) => {
                error!("Failed to write weight to database -> {}", err);
                println!("Failed to write weight to database. Check log for full error.");
            }
        };
    }
}
