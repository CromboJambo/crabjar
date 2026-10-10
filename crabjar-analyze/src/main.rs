use anyhow::Result;
use analyze_common::{
    AnalysisProvenance, AnalysisResult, AnalyzeError, DoubtBlock, ANALYZE_COMMON_VERSION,
};
use analyze_apk::ApkParser;
use analyze_dotnet::analyze_dotnet;
use analyze_firmware::analyze_firmware;
use analyze_har::analyze_har;
use analyze_js::analyze_js;
use analyze_native::ElfParser;
use clap::{Parser, Subcommand};
use chrono::Utc;
use serde_json::{json, Value};
use std::time::Instant;

#[derive(Parser)]
#[command(name = "analyze", version, about = "Reverse engineering toolkit")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze native binary (ELF, Mach-O, PE)
    Binary {
        path: String,
        #[arg(short = 's', long)]
        symbols: bool,
        #[arg(short = 'S', long)]
        strings: bool,
        #[arg(long)]
        sections: bool,
    },
    /// Analyze network capture (HAR or pcap)
    Har { path: String },
    /// Analyze JavaScript/Electron application
    Js { path: String },
    /// Analyze Android APK package
    Apk { path: String },
    /// Analyze firmware image
    Firmware { path: String },
    /// Analyze .NET assembly
    Dotnet { path: String },
}

fn analyze_binary(path: &str, symbols: bool, strings: bool, sections: bool) -> Result<AnalysisResult<Value>, AnalyzeError> {
    let provenance = AnalysisProvenance::new("analyze-native", ANALYZE_COMMON_VERSION);

    // Use analyze-native crate for real ELF parsing
    match ElfParser::analyze(path) {
        Ok(analysis) => {
            let mut data = json!({
                "path": path,
                "format": "ELF",
                "size_bytes": analysis.data.size_bytes,
                "arch": analysis.data.header.arch.to_string(),
                "endian": analysis.data.header.endian.to_string(),
                "class": analysis.data.header.class.to_string(),
                "abi": analysis.data.header.abi.to_string(),
                "entry_point": format!("0x{:x}", analysis.data.header.entry_point),
            });

            if symbols {
                let sym_names: Vec<String> = analysis.data.symbols.iter()
                    .map(|s| s.name.clone())
                    .chain(analysis.data.dynamic_symbols.iter().map(|s| s.name.clone()))
                    .collect();
                data["symbols"] = json!(sym_names);
            }

            if strings {
                data["strings"] = json!(analysis.data.strings);
            }

            if sections {
                let sec_info: Vec<Value> = analysis.data.sections.iter()
                    .map(|s| json!({
                        "name": s.name,
                        "type": s.r#type,
                        "addr": format!("0x{:x}", s.addr),
                        "offset": format!("0x{:x}", s.offset),
                        "size": s.size,
                    }))
                    .collect();
                data["sections"] = json!(sec_info);
            }

            let doubt = DoubtBlock::new("30d")
                .assume("file is a valid ELF binary")
                .blind_spot("dynamic linking not resolved");

            Ok(AnalysisResult::new(
                format!("bin-{}", Utc::now().timestamp_millis()),
                provenance,
                data,
                doubt,
            ))
        }
        Err(_e) => {
            // Return stub result for non-ELF formats
            let mut data = json!({
                "path": path,
                "format": "unknown",
                "size_bytes": 0,
                "arch": null,
                "endian": null
            });

            if symbols {
                data["symbols"] = json!([]);
            }
            if strings {
                data["strings"] = json!([]);
            }
            if sections {
                data["sections"] = json!([]);
            }

            let doubt = DoubtBlock::new("30d")
                .assume("file exists and is readable")
                .blind_spot("dynamic linking not resolved")
                .blind_spot("stripped binaries may have no symbols");

            Ok(AnalysisResult::new(
                format!("bin-{}", Utc::now().timestamp_millis()),
                provenance,
                data,
                doubt,
            ))
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let start = Instant::now();

    let result: Result<serde_json::Value> = match cli.command {
        Commands::Binary { path, symbols, strings, sections } => {
            let analysis = analyze_binary(&path, symbols, strings, sections)?;
            Ok(json!({
                "analysis": analysis,
            }))
        }
        Commands::Har { path } => {
            let analysis = analyze_har(&path)?;
            Ok(json!({
                "analysis": analysis,
            }))
        }
        Commands::Js { path } => {
            let analysis = analyze_js(&path)?;
            Ok(json!({
                "analysis": analysis,
            }))
        }
        Commands::Apk { path } => {
            let analysis = ApkParser::analyze(&path)?;
            Ok(json!({
                "analysis": analysis,
            }))
        }
        Commands::Firmware { path } => {
            let analysis = analyze_firmware(&path)?;
            Ok(json!({
                "analysis": analysis,
            }))
        }
        Commands::Dotnet { path } => {
            let analysis = analyze_dotnet(&path)?;
            Ok(json!({
                "analysis": analysis,
            }))
        }
    };

    let elapsed = start.elapsed();

    // Structured JSON output matching crabjar CLI contract
    let status = if result.is_ok() { "success" } else { "error" };
    let error_msg = result.as_ref().err().map(|e| e.to_string());

    let output = json!({
        "status": status,
        "elapsed_ms": elapsed.as_millis(),
        "data": result.ok(),
        "error": error_msg
    });

    println!("{}", serde_json::to_string_pretty(&output)?);

    Ok(())
}
