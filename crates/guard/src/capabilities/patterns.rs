//! Pattern matching types for capability scoping.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Path pattern for scope matching.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PathPattern(String);

impl PathPattern {
    pub fn new(pattern: impl Into<String>) -> Self {
        Self(pattern.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Check if a path matches this pattern (simple prefix matching).
    pub fn matches(&self, path: &str) -> bool {
        let pattern = self.0.trim_end_matches('*');
        path.starts_with(pattern)
    }
}

impl fmt::Display for PathPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Table pattern for database scope matching.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TablePattern(String);

impl TablePattern {
    pub fn new(pattern: impl Into<String>) -> Self {
        Self(pattern.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Check if a table name matches this pattern (simple prefix matching).
    pub fn matches(&self, table: &str) -> bool {
        let pattern = self.0.trim_end_matches('*');
        table.starts_with(pattern)
    }
}

/// Key pattern for config/environment key matching.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyPattern(String);

impl KeyPattern {
    pub fn new(pattern: impl Into<String>) -> Self {
        Self(pattern.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Check if a key matches this pattern (simple prefix matching).
    pub fn matches(&self, key: &str) -> bool {
        let pattern = self.0.trim_end_matches('*');
        key.starts_with(pattern)
    }
}

/// Endpoint pattern for API endpoint matching.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EndpointPattern(String);

impl EndpointPattern {
    pub fn new(pattern: impl Into<String>) -> Self {
        Self(pattern.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Check if an endpoint matches this pattern (simple prefix matching).
    pub fn matches(&self, endpoint: &str) -> bool {
        let pattern = self.0.trim_end_matches('*');
        endpoint.starts_with(pattern)
    }
}

/// Scope for a data grant — which paths/tables/keys/endpoints are accessible.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityScope {
    pub paths: Vec<PathPattern>,
    pub tables: Vec<TablePattern>,
    pub keys: Vec<KeyPattern>,
    pub endpoints: Vec<EndpointPattern>,
}

impl CapabilityScope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_paths(mut self, paths: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.paths = paths.into_iter().map(PathPattern::new).collect();
        self
    }

    pub fn with_tables(mut self, tables: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tables = tables.into_iter().map(TablePattern::new).collect();
        self
    }

    /// Check if a path is within this scope.
    pub fn path_in_scope(&self, path: &str) -> bool {
        self.paths.iter().any(|p| p.matches(path))
    }

    /// Check if a table is within this scope.
    pub fn table_in_scope(&self, table: &str) -> bool {
        self.tables.iter().any(|t| t.matches(table))
    }

    /// Intersect two scopes — the result contains only what's in both.
    pub fn intersect(&self, other: &CapabilityScope) -> CapabilityScope {
        // For simplicity, intersection keeps patterns that appear in both sets.
        // A more sophisticated implementation would do set-theoretic intersection
        // of the matched spaces.
        let paths: Vec<PathPattern> = self
            .paths
            .iter()
            .filter(|p| other.paths.iter().any(|op| op.as_str() == p.as_str()))
            .cloned()
            .collect();

        let tables: Vec<TablePattern> = self
            .tables
            .iter()
            .filter(|t| other.tables.iter().any(|ot| ot.as_str() == t.as_str()))
            .cloned()
            .collect();

        CapabilityScope {
            paths,
            tables,
            keys: vec![],      // Key intersection not implemented yet
            endpoints: vec![], // Endpoint intersection not implemented yet
        }
    }
}
