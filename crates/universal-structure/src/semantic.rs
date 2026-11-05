//! Semantic annotation types universal across languages.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorHandling {
    Propagated,
    Recovered,
    Ignored,
    Panicked,
}

impl ErrorHandling {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorHandling::Propagated => "Propagated",
            ErrorHandling::Recovered => "Recovered",
            ErrorHandling::Ignored => "Ignored",
            ErrorHandling::Panicked => "Panicked",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mutability {
    Mutable,
    Immutable,
}

impl Mutability {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Mutability::Mutable => "Mutable",
            Mutability::Immutable => "Immutable",
        }
    }
}

/// Ownership mode annotation (Rust-specific patterns).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OwnershipMode {
    Owned,
    Borrowed,
    RefCounted,
    Boxed,
}

impl OwnershipMode {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            OwnershipMode::Owned => "Owned",
            OwnershipMode::Borrowed => "Borrowed",
            OwnershipMode::RefCounted => "RefCounted",
            OwnershipMode::Boxed => "Boxed",
        }
    }
}
