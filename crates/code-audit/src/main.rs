//! Bug Bounty Hunter CLI
//!
//! Usage: bug-bounty-hunter <target> [--type web|binary|code] [--output report.md]

use code_audit::{HuntConfig, TargetType, hunt};
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return;
    }

    let target = &args[1];

    // Parse options
    let mut target_type = TargetType::Unknown;
    let mut output_file = None;
    let mut use_inference = true;

    for (i, arg) in args.iter().enumerate() {
        if *arg == "--type" && i + 1 < args.len() {
            match args[i + 1].as_str() {
                "web" => target_type = TargetType::WebApp,
                "binary" => target_type = TargetType::Binary,
                "code" => target_type = TargetType::SourceCode,
                _ => eprintln!("Unknown target type: {}", args[i + 1]),
            }
        } else if *arg == "--output" && i + 1 < args.len() {
            output_file = Some(args[i + 1].clone());
        } else if *arg == "--no-inference" {
            use_inference = false;
        }
    }

    let config = HuntConfig {
        target: target.clone(),
        target_type,
        use_inference,
        ..Default::default()
    };

    println!("Bug Bounty Hunter v{}", env!("CARGO_PKG_VERSION"));
    println!("Target: {}", target);
    println!("Analyzing...\n");

    let report = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(hunt(config))
        .expect("Hunt failed");

    // Print summary
    let summary = report.summary();
    println!("{}", summary);

    // Generate output
    if let Some(output) = output_file {
        let md = code_audit::report::generate_markdown_report(&report);
        fs::write(&output, &md).expect("Failed to write report");
        println!("\nReport written to: {}", output);
    }
}

fn print_usage() {
    println!("Bug Bounty Hunter v{}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Usage: bug-bounty-hunter <target> [options]");
    println!();
    println!("Options:");
    println!("  --type web|binary|code   Specify target type (auto-detect if omitted)");
    println!("  --output FILE            Write markdown report to file");
    println!();
    println!("Examples:");
    println!("  bug-bounty-hunter https://example.com --type web --output report.md");
    println!("  bug-bounty-hunter ./my-binary --type binary");
    println!("  bug-bounty-hunter src/app.rs --type code");
}
