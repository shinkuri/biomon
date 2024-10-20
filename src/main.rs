use bp::BP;
use chrono::{TimeZone, Utc};
use fern::Dispatch;
use heartrate::Heartrate;
use ini::configparser::ini::Ini;
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io,
    path::Path,
    str::SplitWhitespace,
    sync::{Arc, RwLock},
    time::Duration,
};
use temperature::Temperature;

use log::{error, info};
use mood::Mood;
use rusqlite::{backup::Backup, params, Connection};
use weight::Weight;

mod ble_hrp;
mod bp;
mod heartrate;
mod mood;
mod temperature;
mod utils;
mod weight;

pub trait Stat {
    fn tables(conn: &Connection);
    fn command(input: &mut SplitWhitespace, conn: &Connection) -> String;
    fn help() -> String;
}

type SectionedConfigMap = HashMap<String, HashMap<String, Option<String>>>;

#[tokio::main]
async fn main() {
    setup_logger().expect("Failed to setup logger");

    let conf = match read_config("biomon.ini") {
        Ok(conf) => conf,
        Err(err) => {
            error!("Failed to read config -> {}", err);
            return;
        }
    };
    let conf = Arc::from(RwLock::from(conf));

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
            "temp" => println!("{}", Temperature::command(&mut input, &conn)),
            "record_hrp" => ble_hrp::start(conf.clone(), &conn).await,
            "ingest_markdown_weight" => ingest_markdown_weight(&mut input, &conn),
            "backup" => println!("{}", backup(&mut input, &conn)),
            "restore" => println!("{}", restore(&mut input)),
            "upgrade_tables" => println!("{}", upgrade_tables(&mut input, &conn)),
            "q" => running = false,
            _ => println!("Unknown command: {}", command),
        }
    }

    match write_config("biomon.ini", conf) {
        Ok(_) => info!("Config saved"),
        Err(err) => error!("Failed to save config -> {}", err),
    };
}

fn help() -> String {
    let mut help = String::new();
    help.push_str("List of commands:\n");
    help.push_str(&Weight::help());
    help.push_str(&BP::help());
    help.push_str(&Mood::help());
    help.push_str(&Heartrate::help());
    help.push_str(&Temperature::help());
    help.push_str(
        "\trecord_hrp - Connects to BLE HRP compatible device and collects heartrate data\n",
    );
    help.push_str("\tingest_markdown_weight <file_path:str>\n");
    help.push_str("\tbackup <backup_path:str> - default: ./biomon.sqlite.bak\n");
    help.push_str("\testore <backup_path:str> - default: ./biomon.sqlite.bak\n");
    help.push_str("\tupgrade_tables <file_path:str>\n");
    help.push_str("\tq -> exit");

    help
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

fn tables(conn: &Connection) {
    let _ = conn
        .execute(
            "CREATE TABLE IF NOT EXISTS instruments (
                    id          INTEGER PRIMARY KEY,
                    metric      TEXT UNIQUE NOT NULL,
                    name        TEXT NOT NULL,
                    introduced  INTEGER NOT NULL,
                    deprecated  INTEGER,
                    tol_min     REAL,
                    tol_max     REAL,
                    notes       TEXT
                );",
            [],
        )
        .map_err(|err| error!("Failed to ensure table 'instruments' exists -> {}", err));
}

fn create_tables(conn: &Connection) {
    tables(conn);
    Weight::tables(conn);
    BP::tables(conn);
    Mood::tables(conn);
    Heartrate::tables(conn);
    Temperature::tables(conn);
}

fn upgrade_tables(input: &mut SplitWhitespace, conn: &Connection) -> String {
    let file = match input.next() {
        Some(file) => file,
        None => {
            error!("Missing migration script path");
            return String::from("Missing migration script path");
        }
    };

    let contents = match fs::read_to_string(file) {
        Ok(contents) => contents,
        Err(err) => {
            error!("Failed to read migration script -> {}", err);
            String::from("Failed to read migration script. Check log for full error.")
        }
    };

    // Remove comments
    let contents = contents
        .lines()
        .filter(|&line| !line.starts_with("--"))
        .fold(String::new(), |mut acc, line| {
            acc.push_str(line);
            acc.push('\n');
            acc
        });

    match conn.execute_batch(&contents) {
        Ok(_) => {
            info!("Database migrated");
            String::from("Database migrated")
        }
        Err(err) => {
            error!("Failed to run database migration script -> {}", err);
            String::from("Failed to run database migration script. Check log for full error.")
        }
    }
}

fn backup(input: &mut SplitWhitespace, conn: &Connection) -> String {
    let mut output = String::new();

    let path = match input.next() {
        Some(param) => param,
        None => {
            output.push_str("Using default destination path ./biomon.sqlite.bak\n");
            "biomon.sqlite.bak"
        }
    };

    let mut backup_conn = match Connection::open(path) {
        Ok(backup_conn) => backup_conn,
        Err(err) => {
            output.push_str(&format!("Failed to create/open backup target\n{}\n", err));
            return output;
        }
    };

    let backup = match Backup::new(conn, &mut backup_conn) {
        Ok(backup) => backup,
        Err(e) => {
            output.push_str(&format!("Failed to initialize backup\n{}\n", e));
            return output;
        }
    };

    match backup.run_to_completion(5, Duration::from_millis(250), None) {
        Ok(_) => output.push_str("Backup done\n"),
        Err(_) => output.push_str("Backup failed\n"),
    };

    output
}

fn restore(input: &mut SplitWhitespace) -> String {
    let mut output = String::new();

    let path = match input.next() {
        Some(param) => param,
        None => {
            output.push_str("Using default source path ./biomon.sqlite.bak\n");
            "biomon.sqlite.bak"
        }
    };

    match fs::remove_file("biomon.sqlite") {
        Ok(_) => output.push_str("Removed database\n"),
        Err(err) => {
            output.push_str(&format!("Failed to remove database\n{}\n", err));
            return output;
        }
    };

    match fs::copy(path, "biomon.sqlite") {
        Ok(_) => output.push_str("Restore done\n"),
        Err(_) => output.push_str("Restore failed\n"),
    };

    output
}

fn read_config(path: &str) -> Result<SectionedConfigMap, String> {
    if !Path::new(path).exists() {
        match OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)
        {
            Ok(_) => info!("Missing file create: biomon.ini"),
            Err(err) => {
                error!("Failed to create missing file: biomon.ini -> {}", err);
                return Err(err.to_string());
            }
        }
    }

    let mut ini = Ini::new();
    ini.load(path)
}

fn write_config(path: &str, conf: Arc<RwLock<SectionedConfigMap>>) -> Result<(), io::Error> {
    let mut ini = Ini::new();

    if let Err(err) = set_with_default(&mut ini, conf, "ble_hrp", "hrp_mac", None) {
        error!(
            "Failed to set config for section 'ble_hrp' and key 'hrp_mac' -> {}",
            err
        );
        return Err(err);
    }

    ini.write(path)
}

fn set_with_default(
    ini: &mut Ini,
    conf: Arc<RwLock<SectionedConfigMap>>,
    section: &str,
    key: &str,
    default: Option<String>,
) -> Result<(), io::Error> {
    let conf = conf.read().map_err(|err| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to aquire lock on config map -> {}", err),
        )
    })?;

    let value = conf
        .get(section)
        .and_then(|secmap| secmap.get(key))
        .and_then(|v| v.as_ref())
        .map_or(default.clone(), |v| Some(v.clone()));

    ini.set(section, key, value);
    Ok(())
}

fn ingest_markdown_weight(input: &mut SplitWhitespace, conn: &Connection) {
    let file = match input.next() {
        Some(file) => file,
        None => return,
    };

    let contents = match fs::read_to_string(file) {
        Ok(contents) => contents,
        Err(err) => {
            error!("Failed to read weight file -> {}", err);
            return;
        }
    };

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
