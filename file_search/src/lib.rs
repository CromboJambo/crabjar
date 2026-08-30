//! File Search Engine — BM25-based full-text search over project files.
//!
//! Provides incremental file indexing with git-aware traversal and fast keyword/fuzzy/path queries.

mod indexer;
mod storage;

pub use indexer::{FileIndexer, IndexConfig};
pub use storage::SearchStorage;

/// Default file extensions to index (common source code + config files).
pub const DEFAULT_EXTENSIONS: &[&str] = &[
    // Source code
    "rs",
    "py",
    "go",
    "ts",
    "js",
    "jsx",
    "tsx",
    "java",
    "c",
    "cpp",
    "h",
    "hpp",
    "cs",
    "rb",
    "swift",
    "kt",
    "scala",
    "sh",
    "bash",
    "zsh",
    "fish",
    // Config/data
    "toml",
    "yaml",
    "yml",
    "json",
    "xml",
    "ini",
    "cfg",
    "conf",
    "env",
    "gitignore",
    "dockerignore",
    "editorconfig",
    // Docs
    "md",
    "rst",
    "txt",
    "markdown",
];

/// Default patterns to ignore during indexing.
pub const DEFAULT_IGNORE_PATTERNS: &[&str] = &[
    ".git/",
    "target/",
    "node_modules/",
    "__pycache__/",
    "*.pyc",
    "*.pyo",
    "*.egg-info/",
    ".venv/",
    "venv/",
    ".tox/",
    ".mypy_cache/",
    ".pytest_cache/",
    ".ruff_cache/",
    ".cargo/registry/",
    ".cargo/git/",
    "dist/",
    "build/",
    "*.so",
    "*.dylib",
    "*.dll",
    "*.exe",
];
