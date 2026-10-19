//! Phase 1: Structural region evaluation for JEV.
//!
//! Bridges pesti-structural-tokenizer output into the JEV decision layer
//! by defining an EvaluationTarget type that wraps structural tokens with
//! semantic annotations, then evaluates them against JEV criteria files.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Span kind from pesti-structural-tokenizer (mirrored here to avoid direct dep).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpanKind {
    FnDecl,
    StructDecl,
    EnumDecl,
    ImplBlock,
    LetStmt,
    CallExpr,
    BinaryExpr,
    UnaryExpr,
    LitInt,
    LitFloat,
    LitStr,
    LitBool,
    Ident,
    Keyword,
    Punct,
    Block,
    Item,
    Stmt,
    Expr,
    Pattern,
    Type,
    GenericParam,
    TraitItem,
    MacroCall,
}

impl SpanKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SpanKind::FnDecl => "FnDecl",
            SpanKind::StructDecl => "StructDecl",
            SpanKind::EnumDecl => "EnumDecl",
            SpanKind::ImplBlock => "ImplBlock",
            SpanKind::LetStmt => "LetStmt",
            SpanKind::CallExpr => "CallExpr",
            SpanKind::BinaryExpr => "BinaryExpr",
            SpanKind::UnaryExpr => "UnaryExpr",
            SpanKind::LitInt => "LitInt",
            SpanKind::LitFloat => "LitFloat",
            SpanKind::LitStr => "LitStr",
            SpanKind::LitBool => "LitBool",
            SpanKind::Ident => "Ident",
            SpanKind::Keyword => "Keyword",
            SpanKind::Punct => "Punct",
            SpanKind::Block => "Block",
            SpanKind::Item => "Item",
            SpanKind::Stmt => "Stmt",
            SpanKind::Expr => "Expr",
            SpanKind::Pattern => "Pattern",
            SpanKind::Type => "Type",
            SpanKind::GenericParam => "GenericParam",
            SpanKind::TraitItem => "TraitItem",
            SpanKind::MacroCall => "MacroCall",
        }
    }
}

/// Semantic annotation for ownership.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OwnershipMode {
    Owned,
    Borrowed,
    RefCounted,
    Boxed,
}

impl OwnershipMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            OwnershipMode::Owned => "Owned",
            OwnershipMode::Borrowed => "Borrowed",
            OwnershipMode::RefCounted => "RefCounted",
            OwnershipMode::Boxed => "Boxed",
        }
    }
}

/// Semantic annotation for error handling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorHandling {
    Propagated,
    Recovered,
    Ignored,
    Panicked,
}

impl ErrorHandling {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorHandling::Propagated => "Propagated",
            ErrorHandling::Recovered => "Recovered",
            ErrorHandling::Ignored => "Ignored",
            ErrorHandling::Panicked => "Panicked",
        }
    }
}

/// Semantic annotation for mutability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mutability {
    Mutable,
    Immutable,
}

impl Mutability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Mutability::Mutable => "Mutable",
            Mutability::Immutable => "Immutable",
        }
    }
}

/// A code region that can be evaluated by JEV.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationTarget {
    /// The structural span kind (what this region is).
    pub structure: SpanKind,
    /// Byte offsets in the source file.
    pub range: [usize; 2],
    /// Source text for embedding fallback.
    pub source: String,
    /// Function name if applicable.
    #[serde(default)]
    pub name: Option<String>,
    /// Semantic tags from analyzers (Phase 1: empty).
    #[serde(default)]
    pub semantic_tags: HashMap<String, String>,
}

impl EvaluationTarget {
    /// Convert to an observation string for JEV embedding evaluation.
    pub fn to_observation(&self) -> String {
        let mut obs = format!("[{}] ", self.structure.as_str());

        if let Some(name) = &self.name {
            obs.push_str(&format!("{} - ", name));
        }

        // Include semantic tags in observation for richer embedding context
        if !self.semantic_tags.is_empty() {
            let tags: Vec<String> = self
                .semantic_tags
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            obs.push_str(&format!("({}) ", tags.join(", ")));
        }

        obs.push_str(&self.source);
        obs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_kind_serialization() {
        let kind = SpanKind::FnDecl;
        assert_eq!(kind.as_str(), "FnDecl");
    }

    #[test]
    fn test_evaluation_target_creation() {
        let target = EvaluationTarget {
            structure: SpanKind::FnDecl,
            range: [0, 100],
            source: "fn foo() {} ".to_string(),
            name: Some("foo".to_string()),
            semantic_tags: HashMap::new(),
        };

        assert_eq!(target.structure, SpanKind::FnDecl);
        assert_eq!(target.name.as_deref(), Some("foo"));
    }

    #[test]
    fn test_observation_string() {
        let mut tags = HashMap::new();
        tags.insert("ownership".to_string(), "Owned".to_string());

        let target = EvaluationTarget {
            structure: SpanKind::FnDecl,
            range: [0, 100],
            source: "fn foo() {} ".to_string(),
            name: Some("foo".to_string()),
            semantic_tags: tags,
        };

        let obs = target.to_observation();
        assert!(obs.starts_with("[FnDecl] foo"));
        assert!(obs.contains("ownership=Owned"));
        assert!(obs.contains("fn foo() {} "));
    }
}
