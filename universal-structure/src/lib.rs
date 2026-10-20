//! Universal structural token representation for multi-language code analysis.
//!
//! All languages map to a common set of structural tokens regardless of syntax
//! differences. Language-specific tags are preserved alongside universal tokens
//! for nuanced analysis while maintaining cross-language compatibility.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod adapters;

/// Universal span kind covering major language constructs across all languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpanKind {
    // Declarations
    FnDecl,     // Function declaration (any language)
    ClassDecl,  // Class/type declaration (replaces Rust-specific StructDecl/EnumDecl)
    MethodDecl, // Method in object-oriented contexts
    ModuleDecl, // Module/package declaration
    LetStmt,    // Variable binding/declaration
    ConstStmt,  // Constant declaration

    // Expressions
    CallExpr,   // Function/method call
    BinaryExpr, // Binary operation (a + b)
    UnaryExpr,  // Unary operation (!x, -x)
    LitInt,     // Integer literal
    LitFloat,   // Floating point literal
    LitStr,     // String literal
    LitBool,    // Boolean literal
    Ident,      // Identifier
    Keyword,    // Language keyword
    Punct,      // Punctuation

    // Blocks and structure
    Block,        // Code block ({ ... })
    Item,         // Top-level item
    Stmt,         // Statement
    Expr,         // Expression
    Pattern,      // Match pattern
    Type,         // Type annotation/declaration
    GenericParam, // Generic type parameter
    TraitItem,    // Trait/interface member

    // Module boundaries (universal)
    ImportStatement, // Import/include statement
    ExportStatement, // Export declaration

    // Error handling patterns (universal)
    TryBlock,     // Try block
    CatchBlock,   // Catch/except block
    FinallyBlock, // Finally block
    ThrowExpr,    // Throw/raise expression

    // Control flow (universal)
    IfStmt,       // If statement
    ElseBlock,    // Else branch
    WhileLoop,    // While loop
    ForLoop,      // For loop
    ReturnStmt,   // Return statement
    BreakStmt,    // Break statement
    ContinueStmt, // Continue statement
    SwitchStmt,   // Switch statement
    CaseBlock,    // Case branch

    // Language-specific constructs that map to universal concepts
    MacroCall, // Macro invocation (Rust) or template literal (JS)
    Decorator, // Python decorator / JS class decorator
}

impl SpanKind {
    /// Get the string representation of this span kind.
    pub fn as_str(&self) -> &'static str {
        match self {
            SpanKind::FnDecl => "FnDecl",
            SpanKind::ClassDecl => "ClassDecl",
            SpanKind::MethodDecl => "MethodDecl",
            SpanKind::ModuleDecl => "ModuleDecl",
            SpanKind::LetStmt => "LetStmt",
            SpanKind::ConstStmt => "ConstStmt",
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
            SpanKind::ImportStatement => "ImportStatement",
            SpanKind::ExportStatement => "ExportStatement",
            SpanKind::TryBlock => "TryBlock",
            SpanKind::CatchBlock => "CatchBlock",
            SpanKind::FinallyBlock => "FinallyBlock",
            SpanKind::ThrowExpr => "ThrowExpr",
            SpanKind::IfStmt => "IfStmt",
            SpanKind::ElseBlock => "ElseBlock",
            SpanKind::WhileLoop => "WhileLoop",
            SpanKind::ForLoop => "ForLoop",
            SpanKind::ReturnStmt => "ReturnStmt",
            SpanKind::BreakStmt => "BreakStmt",
            SpanKind::ContinueStmt => "ContinueStmt",
            SpanKind::SwitchStmt => "SwitchStmt",
            SpanKind::CaseBlock => "CaseBlock",
            SpanKind::MacroCall => "MacroCall",
            SpanKind::Decorator => "Decorator",
        }
    }

    /// Check if this is a declaration span.
    pub fn is_declaration(&self) -> bool {
        matches!(
            self,
            SpanKind::FnDecl
                | SpanKind::ClassDecl
                | SpanKind::MethodDecl
                | SpanKind::ModuleDecl
                | SpanKind::LetStmt
                | SpanKind::ConstStmt
        )
    }

    /// Check if this is an expression span.
    pub fn is_expression(&self) -> bool {
        matches!(
            self,
            SpanKind::CallExpr
                | SpanKind::BinaryExpr
                | SpanKind::UnaryExpr
                | SpanKind::LitInt
                | SpanKind::LitFloat
                | SpanKind::LitStr
                | SpanKind::LitBool
                | SpanKind::Ident
                | SpanKind::ThrowExpr
        )
    }

    /// Check if this is a control flow span.
    pub fn is_control_flow(&self) -> bool {
        matches!(
            self,
            SpanKind::IfStmt
                | SpanKind::ElseBlock
                | SpanKind::WhileLoop
                | SpanKind::ForLoop
                | SpanKind::ReturnStmt
                | SpanKind::BreakStmt
                | SpanKind::ContinueStmt
                | SpanKind::SwitchStmt
                | SpanKind::CaseBlock
        )
    }

    /// Check if this is an error handling span.
    pub fn is_error_handling(&self) -> bool {
        matches!(
            self,
            SpanKind::TryBlock
                | SpanKind::CatchBlock
                | SpanKind::FinallyBlock
                | SpanKind::ThrowExpr
        )
    }

    /// Check if this is a module boundary span.
    pub fn is_module_boundary(&self) -> bool {
        matches!(
            self,
            SpanKind::ImportStatement | SpanKind::ExportStatement | SpanKind::ModuleDecl
        )
    }
}

/// Universal structural token with position info and language-specific tags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralToken {
    /// The universal span kind.
    pub kind: SpanKind,
    /// Source text of this token/span.
    pub text: String,
    /// Byte offsets in the source file [start, end).
    pub range: [usize; 2],
    /// Line number (1-indexed) for display purposes.
    #[serde(default)]
    pub line: Option<u32>,
    /// Language-specific tags preserved alongside universal representation.
    #[serde(default)]
    pub tags: HashMap<String, String>,
}

impl StructuralToken {
    pub fn new(kind: SpanKind, text: impl Into<String>, range: [usize; 2]) -> Self {
        Self {
            kind,
            text: text.into(),
            range,
            line: None,
            tags: HashMap::new(),
        }
    }

    pub fn with_line(mut self, line: u32) -> Self {
        self.line = Some(line);
        self
    }

    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }
}

/// Hierarchical span representing a nested code region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    /// The universal span kind.
    pub kind: SpanKind,
    /// Opening token (if any).
    #[serde(default)]
    pub open_token: Option<StructuralToken>,
    /// Child spans within this region.
    #[serde(default)]
    pub body: Vec<Span>,
    /// Closing token (if any).
    #[serde(default)]
    pub close_token: Option<StructuralToken>,
    /// Source text for embedding fallback.
    #[serde(default)]
    pub source: String,
}

impl Span {
    pub fn new(kind: SpanKind) -> Self {
        Self {
            kind,
            open_token: None,
            body: Vec::new(),
            close_token: None,
            source: String::new(),
        }
    }

    /// Get all descendant spans of a specific kind.
    pub fn find_all(&self, target_kind: SpanKind) -> Vec<&Span> {
        let mut results = Vec::new();
        if self.kind == target_kind {
            results.push(self);
        }
        for child in &self.body {
            results.extend(child.find_all(target_kind));
        }
        results
    }

    /// Get the depth of this span tree.
    pub fn depth(&self) -> usize {
        let mut max_child = 0;
        for child in &self.body {
            let child_depth = child.depth();
            if child_depth > max_child {
                max_child = child_depth;
            }
        }
        1 + max_child
    }

    /// Count total spans in this tree.
    pub fn span_count(&self) -> usize {
        let mut count = 1;
        for child in &self.body {
            count += child.span_count();
        }
        count
    }
}

/// Budget configuration for resolution control.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    /// Maximum number of tokens to process.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    /// Minimum node size in bytes before recursing.
    #[serde(default = "default_min_node_bytes")]
    pub min_node_bytes: usize,
}

fn default_max_tokens() -> usize {
    10000
}

fn default_min_node_bytes() -> usize {
    50
}

impl Budget {
    pub fn new(max_tokens: usize, min_node_bytes: usize) -> Self {
        Self {
            max_tokens,
            min_node_bytes,
        }
    }

    pub fn unlimited() -> Self {
        Self::new(usize::MAX, 0)
    }
}

impl Default for Budget {
    fn default() -> Self {
        Budget {
            max_tokens: default_max_tokens(),
            min_node_bytes: default_min_node_bytes(),
        }
    }
}

/// Semantic annotation types universal across languages.
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
    /// The universal structural span kind.
    pub structure: SpanKind,
    /// Byte offsets in the source file.
    pub range: [usize; 2],
    /// Source text for embedding fallback.
    pub source: String,
    /// Function/method name if applicable.
    #[serde(default)]
    pub name: Option<String>,
    /// Language identifier (e.g., "rust", "javascript", "python").
    #[serde(default)]
    pub language: Option<String>,
    /// Semantic tags from analyzers.
    #[serde(default)]
    pub semantic_tags: HashMap<String, String>,
    /// Typed semantic fields.
    #[serde(default)]
    pub error_handling: Option<ErrorHandling>,
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

        // Include typed semantic tags first
        let mut parts = Vec::new();
        if let Some(eh) = &self.error_handling {
            parts.push(format!("error_handling={}", eh.as_str()));
        }
        if let Some(mt) = &self.mutability {
            parts.push(format!("mutability={}", mt.as_str()));
        }

        // Include raw semantic tags for extensibility
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
    pub fn parse_value(&self, field: &str) -> Option<String> {
        match field {
            "structure" => Some(self.structure.as_str().to_string()),
            "name" => self.name.clone(),
            "language" => self.language.clone(),
            "error_handling" | "error_handling_mode" => {
                self.error_handling.as_ref().map(|e| e.as_str().to_string())
            }
            "mutability" | "mutability_mode" => {
                self.mutability.as_ref().map(|m| m.as_str().to_string())
            }
            _ => self.semantic_tags.get(field).cloned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_kind_as_str() {
        assert_eq!(SpanKind::FnDecl.as_str(), "FnDecl");
        assert_eq!(SpanKind::ClassDecl.as_str(), "ClassDecl");
        assert_eq!(SpanKind::ImportStatement.as_str(), "ImportStatement");
        assert_eq!(SpanKind::TryBlock.as_str(), "TryBlock");
    }

    #[test]
    fn test_span_kind_classification() {
        assert!(SpanKind::FnDecl.is_declaration());
        assert!(!SpanKind::CallExpr.is_declaration());

        assert!(SpanKind::CallExpr.is_expression());
        assert!(!SpanKind::FnDecl.is_expression());

        assert!(SpanKind::IfStmt.is_control_flow());
        assert!(!SpanKind::LetStmt.is_control_flow());

        assert!(SpanKind::TryBlock.is_error_handling());
        assert!(!SpanKind::ReturnStmt.is_error_handling());

        assert!(SpanKind::ImportStatement.is_module_boundary());
        assert!(!SpanKind::Expr.is_module_boundary());
    }

    #[test]
    fn test_structural_token_creation() {
        let token = StructuralToken::new(SpanKind::FnDecl, "fn foo()", [0, 10])
            .with_line(1)
            .with_tag("access", "public");

        assert_eq!(token.kind, SpanKind::FnDecl);
        assert_eq!(token.text, "fn foo()");
        assert_eq!(token.range, [0, 10]);
        assert_eq!(token.line, Some(1));
        assert_eq!(token.tags.get("access").unwrap(), &"public");
    }

    #[test]
    fn test_span_tree_operations() {
        let inner = Span::new(SpanKind::LetStmt);
        let outer = Span::new(SpanKind::FnDecl);
        let mut span = Span::new(SpanKind::ModuleDecl);
        span.body.push(outer);
        span.body[0].body.push(inner);

        assert_eq!(span.depth(), 3);
        assert_eq!(span.span_count(), 3);
        assert_eq!(span.find_all(SpanKind::LetStmt).len(), 1);
    }

    #[test]
    fn test_evaluation_target_observation() {
        let target = EvaluationTarget {
            structure: SpanKind::FnDecl,
            range: [0, 100],
            source: "fn foo() {} ".to_string(),
            name: Some("foo".to_string()),
            language: Some("rust".to_string()),
            semantic_tags: HashMap::new(),
            error_handling: Some(ErrorHandling::Propagated),
            mutability: Some(Mutability::Immutable),
        };

        let obs = target.to_observation();
        assert!(obs.starts_with("[FnDecl] foo"));
        assert!(obs.contains("error_handling=Propagated"));
        assert!(obs.contains("mutability=Immutable"));
        assert!(obs.contains("fn foo() {} "));
    }

    #[test]
    fn test_evaluation_target_parse_value() {
        let target = EvaluationTarget {
            structure: SpanKind::ClassDecl,
            range: [0, 100],
            source: "".to_string(),
            name: Some("MyClass".to_string()),
            language: Some("javascript".to_string()),
            semantic_tags: HashMap::new(),
            error_handling: None,
            mutability: None,
        };

        assert_eq!(
            target.parse_value("structure"),
            Some("ClassDecl".to_string())
        );
        assert_eq!(target.parse_value("name"), Some("MyClass".to_string()));
        assert_eq!(
            target.parse_value("language"),
            Some("javascript".to_string())
        );
        assert_eq!(target.parse_value("unknown"), None);
    }

    #[test]
    fn test_budget_defaults() {
        let budget = Budget::default();
        assert_eq!(budget.max_tokens, 10000);
        assert_eq!(budget.min_node_bytes, 50);
    }
}
