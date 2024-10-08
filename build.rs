use std::fs;
use std::path::Path;

fn main() {
    // copy sqlite files to build output
    let dll = "sqlite3.dll";
    let def = "sqlite3.def";
    let out_dir = &std::env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir);

    fs::copy(dll, dest.join(dll)).expect("Failed to copy sqlite.dll to output directory");
    fs::copy(def, dest.join(def)).expect("Failed to copy sqlite.def to output directory");
}