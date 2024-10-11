use std::{env, fs};
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let target = env::var("CARGO_CFG_TARGET_OS").unwrap();

    let windows_sqlite = "sqlite3.dll";
    // let linux_sqlite = "sqlite3";

    let sqlite_source = match target.as_str() {
        "windows" => windows_sqlite,
        // "linux" => linux_sqlite,
        // "macos" => 
        _ => panic!("Unsupported target platform")
    };

    // copy sqlite files to build output
    let dest_path = Path::new(&out_dir);
    fs::copy(sqlite_source, dest_path.join(sqlite_source))
        .unwrap_or_else(|_| panic!("Failed to copy sqlite to output directory for target {}", target));
}