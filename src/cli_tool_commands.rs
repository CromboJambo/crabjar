//! Tool registry management CLI subcommands (split from cli_commands.rs by 500 LoC gate).

use clap::Subcommand;

/// Tool registry management subcommands
#[derive(Debug, Subcommand, Clone)]
pub enum ToolCommand {
    /// List registered tools
    List {
        /// Filter by tool type (command, mcp, llm, vector_store, embedding, rag, skill)
        #[arg(short, long)]
        r#type: Option<String>,
        /// Maximum number of tools to return
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },
    /// Discover tools from known sources
    Discover {
        /// Source name to use for discovery
        #[arg(short, long, default_value = "cli")]
        source: String,
    },
    /// Install a skill from a local path (loads manifest and registers tools)
    SkillInstall {
        /// Path to the skill directory or manifest.json file
        path: String,
    },
    /// Enable a plugin by name
    PluginEnable {
        /// Plugin name
        name: String,
    },
    /// Disable a plugin by name
    PluginDisable {
        /// Plugin name
        name: String,
    },
    /// Unregister (remove) a tool from the registry
    ToolUninstall {
        /// Tool name to remove
        name: String,
    },
    /// Search for CrabJar skills on GitHub by tag
    Search {
        /// Search query (optional — defaults to all crabjar-skill tagged repos)
        query: Option<String>,
        /// Minimum star count filter
        #[arg(long)]
        stars: Option<u32>,
        /// Sort field (stars, updated, name)
        #[arg(long, default_value = "stars")]
        sort: String,
        /// Maximum results to return
        #[arg(long, default_value = "10")]
        limit: u32,
    },
    /// Install a skill from a GitHub repository
    Install {
        /// GitHub repository (owner/repo format)
        repo: String,
        /// Installation directory (default: ~/.crabjar/skills)
        #[arg(long)]
        target: Option<String>,
    },
    /// Search for skills on ClawHub.io registry
    ClawhubSearch {
        /// Search query
        query: Option<String>,
        /// Maximum results to return
        #[arg(long, default_value = "10")]
        limit: u32,
    },
    /// Install a skill from ClawHub.io registry with security scanning
    ClawhubInstall {
        /// Skill name on ClawHub
        name: String,
        /// Version to install (default: latest)
        #[arg(long)]
        version: Option<String>,
        /// Installation directory (default: ~/.crabjar/skills)
        #[arg(long)]
        target: Option<String>,
    },
    /// Scan a skill package for security issues
    Scan {
        /// Path to the skill package directory or zip file
        path: String,
        /// Expected checksum to verify (optional)
        #[arg(long)]
        checksum: Option<String>,
    },
    /// Manage the skill cache
    Cache {
        /// Cache subcommand
        #[command(subcommand)]
        action: ToolCacheCommand,
    },
    /// Check for skill updates
    Updates {
        /// Skill name to check (optional — checks all if omitted)
        name: Option<String>,
    },
    /// Show authentication status for remote registries
    Auth,
}

/// Skill cache management subcommands
#[derive(Debug, Subcommand, Clone)]
pub enum ToolCacheCommand {
    /// List cached skills
    List,
    /// Remove a skill from the cache
    Remove {
        /// Skill name to remove
        name: String,
    },
    /// Clean up stale cache entries (>30 days)
    Clean,
}
