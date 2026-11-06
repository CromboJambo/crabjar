//! cargo-artifact: Content-addressed artifact packaging for Rust projects.
//!
//! Usage:
//!   cargo artifact build [--target <triple>] [-o <output>] [path]
//!   cargo artifact inspect <artifact.ca>
//!   cargo artifact install <artifact.ca> [--prefix <dir>]
//!   cargo artifact verify <artifact.ca>

use anyhow::Result;
use clap::{Parser, Subcommand};
use crabjar_artifact::{
    build_artifact, inspect_artifact, verify_artifact, write_artifact_metadata,
};
use std::path::PathBuf;

#[derive(Parser)]
#[clap(
    name = "cargo-artifact",
    about = "Content-addressed artifact packaging for Rust"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build a cargo project into a content-addressed artifact
    Build {
        /// Path to the Cargo project (default: current directory)
        #[clap(default_value = ".")]
        path: PathBuf,

        /// Target triple (default: x86_64-unknown-linux-musl)
        #[clap(long)]
        target: Option<String>,

        /// Output .ca file path (default: <name>.ca)
        #[clap(short = 'o', long)]
        output: Option<PathBuf>,
    },

    /// Inspect an artifact and show its metadata
    Inspect {
        /// Path to the .ca artifact file
        path: PathBuf,
    },

    /// Install an artifact to a target location
    Install {
        /// Path to the .ca artifact file
        path: PathBuf,

        /// Installation prefix (default: /opt)
        #[clap(short = 'p', long, default_value = "/opt")]
        prefix: PathBuf,
    },

    /// Verify artifact integrity
    Verify {
        /// Path to the .ca artifact file
        path: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build {
            path,
            target,
            output,
        } => {
            println!("Building artifact for project at {}", path.display());

            let metadata = build_artifact(&path, target.as_deref())?;

            // Determine output path
            let output_path =
                output.unwrap_or_else(|| PathBuf::from(format!("{}.ca", metadata.name)));

            write_artifact_metadata(&output_path, &metadata)?;

            println!("Artifact built successfully!");
            println!("  Name: {}", metadata.name);
            println!("  Hash: {}", metadata.hash);
            println!("  Target: {}", metadata.target);
            println!("  Output: {}", output_path.display());
        }

        Commands::Inspect { path } => {
            let metadata = inspect_artifact(&path)?;

            println!("Artifact: {}", metadata.name);
            println!("Hash: {}", metadata.hash);
            println!("Target: {}", metadata.target);
            println!("Compiler: {}", metadata.inputs.compiler_version);
            if let Some(binary) = &metadata.outputs.binary {
                println!("Binary: {}", binary);
            }
        }

        Commands::Install { path, prefix } => {
            let metadata = inspect_artifact(&path)?;

            // Verify before installing
            if !verify_artifact(&path)? {
                anyhow::bail!("Artifact verification failed");
            }

            println!("Installing {} to {}", metadata.name, prefix.display());
            // TODO: actual installation logic (extract binary, set permissions)
        }

        Commands::Verify { path } => {
            let valid = verify_artifact(&path)?;
            if valid {
                println!("Artifact is valid");
            } else {
                anyhow::bail!("Artifact verification failed");
            }
        }
    }

    Ok(())
}
