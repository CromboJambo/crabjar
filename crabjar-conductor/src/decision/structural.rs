//! Phase 1: Structural region evaluation for JEV.
//!
//! Bridges pesti-structural-tokenizer output into the JEV decision layer
//! by defining an EvaluationTarget type that wraps structural tokens with
//! semantic annotations, then evaluates them against JEV criteria files.

use crate::decision::condition::evaluate_condition;
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

    // Typed semantic fields (Phase 2: populated by analyzers)
    /// Ownership mode detected by analyzer.
    #[serde(default)]
    pub ownership_mode: Option<OwnershipMode>,
    /// Error handling pattern detected by analyzer.
    #[serde(default)]
    pub error_handling: Option<ErrorHandling>,
    /// Mutability pattern detected by analyzer.
    #[serde(default)]
    pub mutability: Option<Mutability>,
}

impl EvaluationTarget {
    /// Convert to an observation string for JEV embedding evaluation.
    pub fn to_observation(&self) -> String {
        let mut obs = format!("[{}] ", self.structure.as_str());

        if let Some(name) = &self.name {
            obs.push_str(&format!("{} - ", name));
        }

        // Include typed semantic tags first (Phase 2)
        let mut parts = Vec::new();
        if let Some(om) = &self.ownership_mode {
            parts.push(format!("ownership={}", om.as_str()));
        }
        if let Some(eh) = &self.error_handling {
            parts.push(format!("error_handling={}", eh.as_str()));
        }
        if let Some(mt) = &self.mutability {
            parts.push(format!("mutability={}", mt.as_str()));
        }

        // Include raw semantic tags for extensibility (Phase 1)
        for (k, v) in &self.semantic_tags {
            parts.push(format!("{}={}", k, v));
        }

        if !parts.is_empty() {
            obs.push_str(&format!("({}) ", parts.join(", ")));
        }

        obs.push_str(&self.source);
        obs
    }

    /// Resolve a field name to its string value for condition evaluation.
    /// Handles both legacy string fields and Phase 2 typed semantic fields.
    pub fn parse_value(&self, field: &str) -> Option<String> {
        match field {
            "structure" => Some(self.structure.as_str().to_string()),
            "name" => self.name.clone(),
            "ownership" | "ownership_mode" => self.ownership_mode.as_ref().map(|m| m.as_str().to_string()),
            "error_handling" | "error_handling_mode" => self.error_handling.as_ref().map(|e| e.as_str().to_string()),
            "mutability" | "mutability_mode" => self.mutability.as_ref().map(|m| m.as_str().to_string()),
            _ => self.semantic_tags.get(field).cloned(),
        }
    }
}

/// Build an EvaluationTarget from a code region, populating typed semantic fields
/// by analyzing the source text for ownership, error handling, and mutability patterns.
pub fn build_target(
    structure: SpanKind,
    range: [usize; 2],
    source: &str,
    name: Option<&str>,
) -> EvaluationTarget {
    let mut target = EvaluationTarget {
        structure,
        range,
        source: source.to_string(),
        name: name.map(|s| s.to_string()),
        semantic_tags: HashMap::new(),
        ownership_mode: None,
        error_handling: None,
        mutability: None,
    };

    // Phase 2: Analyze source text for typed semantic fields

    // Error handling patterns
    if source.contains("unwrap()") {
        target.error_handling = Some(ErrorHandling::Panicked);
    } else if source.contains(".map_err(") || source.contains("with_context") {
        target.error_handling = Some(ErrorHandling::Recovered);
    } else if source.contains("return Err") || source.contains("?") {
        target.error_handling = Some(ErrorHandling::Propagated);
    }

    // Ownership patterns
    if source.contains("std::rc::Rc") || source.contains("Arc<") {
        target.ownership_mode = Some(OwnershipMode::RefCounted);
    } else if source.contains("Box<") {
        target.ownership_mode = Some(OwnershipMode::Boxed);
    }

    // Mutability patterns
    if source.contains("mut ") || source.contains(".push(") || source.contains(".insert(") {
        target.mutability = Some(Mutability::Mutable);
    } else {
        target.mutability = Some(Mutability::Immutable);
    }

    // Also populate legacy string fields for backward compatibility
    if let Some(eh) = &target.error_handling {
        target.semantic_tags.insert("error_handling".to_string(), eh.as_str().to_string());
    }
    if let Some(om) = &target.ownership_mode {
        target.semantic_tags.insert("ownership_mode".to_string(), om.as_str().to_string());
    }
    if let Some(mt) = &target.mutability {
        target.semantic_tags.insert("mutability".to_string(), mt.as_str().to_string());
    }

    target
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
            ownership_mode: None,
            error_handling: None,
            mutability: None,
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
            ownership_mode: None,
            error_handling: None,
            mutability: None,
        };

        let obs = target.to_observation();
        assert!(obs.starts_with("[FnDecl] foo"));
        assert!(obs.contains("ownership=Owned"));
        assert!(obs.contains("fn foo() {} "));
    }

    #[test]
    fn test_observation_string_with_typed_tags() {
        let target = EvaluationTarget {
            structure: SpanKind::FnDecl,
            range: [0, 100],
            source: "fn foo() {} ".to_string(),
            name: Some("foo".to_string()),
            semantic_tags: HashMap::new(),
            ownership_mode: Some(OwnershipMode::Borrowed),
            error_handling: Some(ErrorHandling::Propagated),
            mutability: Some(Mutability::Immutable),
        };

        let obs = target.to_observation();
        assert!(obs.contains("ownership=Borrowed"));
        assert!(obs.contains("error_handling=Propagated"));
        assert!(obs.contains("mutability=Immutable"));
    }
}