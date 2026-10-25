//! Crabjar Commit CLI Tool
//!
//! Generate conventional commit messages from git diffs and optionally
//! stage files and create commits.

use clap::{Parser, Subcommand};
use crabjar_commit::CommitGenerator;
use crabjar_git::GitRepo;
use std::path::PathBuf;

#[derive(Parser)]
#[clap(name = "crabjar-commit", about = "Generate conventional commit messages from git diffs")]
struct Cli {
    /// Path to git repository (defaults to current directory)
    #[clap(short, long, default_value = ".")]
    repo: PathBuf,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a commit message from current changes
    Generate {
        /// Output as JSON
        #[clap(short, long)]
        json: bool,
    },
    /// Show git status (staged, unstaged, untracked)
    Status {
        /// Output as JSON
        #[clap(short, long)]
        json: bool,
    },
    /// Stage specific files
    Add {
        /// Files to stage
        paths: Vec<String>,
    },
    /// Stage all changes
    AddAll,
    /// Create a commit with generated message
    Commit {
        /// Override the generated commit message
        #[clap(short, long)]
        message: Option<String>,
        /// Author name (defaults to git config)
        #[clap(long)]
        author_name: Option<String>,
        /// Author email (defaults to git config)
        #[clap(long)]
        author_email: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate { json } => {
            let repo = GitRepo::open(&cli.repo).expect("Failed to open repository");
            let generator = CommitGenerator::new(&repo);

            match generator.generate() {
                Ok(msg) => {
                    if json {
                        println!("{}", serde_json::to_string_pretty(&msg).unwrap());
                    } else {
                        println!("{}", msg);
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Status { json } => {
            let repo = GitRepo::open(&cli.repo).expect("Failed to open repository");

            match repo.status() {
                Ok(status) => {
                    if json {
                        println!("{}", serde_json::to_string_pretty(&status).unwrap());
                    } else {
                        println!("Staged: {}", status.staged.len());
                        for file in &status.staged {
                            println!("  + {}", file.path);
                        }
                        println!("Unstaged: {}", status.unstaged.len());
                        for file in &status.unstaged {
                            if file.modified {
                                println!("  M {}", file.path);
                            } else if file.new_file {
                                println!("  A {}", file.path);
                            } else if file.deleted {
                                println!("  D {}", file.path);
                            }
                        }
                        println!("Untracked: {}", status.untracked.len());
                        for file in &status.untracked {
                            println!("  ? {}", file.path);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Add { paths } => {
            let repo = GitRepo::open(&cli.repo).expect("Failed to open repository");

            match repo.add(&paths) {
                Ok(_) => println!("Staged {} file(s)", paths.len()),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::AddAll => {
            let repo = GitRepo::open(&cli.repo).expect("Failed to open repository");

            match repo.add_all() {
                Ok(_) => println!("Staged all changes"),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Commit {
            message,
            author_name,
            author_email,
        } => {
            let repo = GitRepo::open(&cli.repo).expect("Failed to open repository");

            // Generate or use provided message
            let commit_msg = match message {
                Some(msg) => msg,
                None => {
                    let generator = CommitGenerator::new(&repo);
                    match generator.generate() {
                        Ok(msg) => {
                            println!("Generated: {}", msg);
                            msg.to_string()
                        }
                        Err(e) => {
                            eprintln!("Error generating message: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
            };

            // Use git config defaults if not provided
            let author = author_name.unwrap_or_else(|| "CrabJar".to_string());
            let email = author_email.unwrap_or_else(|| "crabjar@example.com".to_string());

            match repo.commit(&commit_msg, &author, &email) {
                Ok(result) => {
                    println!("Committed: {}", result.oid);
                    println!("Message: {}", result.message);
                    println!("Files changed: {}", result.files_changed);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
