//! CLI tool: strip secrets from training data files.
//!
//! Usage: crabjar-train-secret-stripper <input> [-o <output>] [--config <json>]

use clap::Parser;
use crabjar_train_secret_stripper::{strip_with_config, StripConfig};
use std::fs;

#[derive(Parser)]
#[command(name = "crabjar-train-secret-stripper")]
#[command(about = "Strip secret-shaped objects from training data files")]
struct Args {
    /// Input file to process
    input: String,

    /// Output file (default: <input>.cleaned)
    #[arg(short, long)]
    output: Option<String>,

    /// Path to JSON config file with custom patterns
    #[arg(long)]
    config: Option<String>,

    /// Print JSON report of secrets found to stdout
    #[arg(long)]
    report: bool,
}

fn main() {
    let args = Args::parse();

    // Load config
    let config = if let Some(config_path) = &args.config {
        let content = fs::read_to_string(config_path).expect("Failed to read config file");
        serde_json::from_str(&content).expect("Failed to parse config JSON")
    } else {
        StripConfig::default()
    };

    // Read input
    let text = fs::read_to_string(&args.input).expect("Failed to read input file");

    // Strip secrets
    let result = strip_with_config(&text, &config).expect("Failed to strip secrets");

    // Determine output path
    let output_path = args.output.unwrap_or_else(|| {
        let stem = args
            .input
            .rsplit_once('.')
            .map(|(s, _)| s)
            .unwrap_or(&args.input);
        format!("{}.cleaned", stem)
    });

    // Write cleaned output
    fs::write(&output_path, &result.cleaned).expect("Failed to write output file");

    if args.report {
        let report = serde_json::json!({
            "input": args.input,
            "output": output_path,
            "secrets_found": result.secrets_found,
            "total_secrets": result.secrets_found.values().sum::<usize>()
        });
        println!("{}", report.to_string());
    } else {
        println!("Cleaned: {} → {}", args.input, output_path);
        if !result.secrets_found.is_empty() {
            let total: usize = result.secrets_found.values().sum();
            println!("Stripped {} secret(s):", total);
            for (r#type, count) in &result.secrets_found {
                println!("  {}: {}", r#type, count);
            }
        } else {
            println!("No secrets found.");
        }
    }
}
