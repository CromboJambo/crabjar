use std::env;
use std::fs;
use std::path::PathBuf;
use serde_json::json;

fn main() {
    let args = env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        eprintln!("Usage: hermes-struct-tok-pipeline watch <dir> | process <file>");
        std::process::exit(1);
    }

    match args[1].as_str() {
        "watch" => {
            let watch_dir = PathBuf::from(&args[2]);
            println!("Watching {} for Rust source files...", watch_dir.display());
            // In real impl: use notify crate to watch for new .rs files
            process_directory(&watch_dir);
        }
        "process" => {
            let file = &args[2];
            process_file(file);
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            std::process::exit(1);
        }
    }
}

fn process_directory(dir: &PathBuf) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension() == Some(std::ffi::OsStr::new("rs")) {
                process_file(path.to_str().unwrap());
            }
        }
    }
}

fn process_file(file: &str) {
    println!("Processing: {}", file);
    // Call struct-tok binary via std::process::Command
    let output = std::process::Command::new("struct-tok")
        .args(["encode", "--file"])
        .arg(file)
        .output();
    
    match output {
        Ok(o) if o.status.success() => {
            let tokens = String::from_utf8_lossy(&o.stdout);
            let event = json!({
                "type": "rust_source",
                "file": file,
                "tokens": tokens.trim()
            });
            println!("{}", event.to_string());
        }
        _ => {
            eprintln!("Failed to tokenize {}", file);
        }
    }
}
