//! Universal span kinds covering major language constructs.

use serde::{Deserialize, Serialize};

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
    LitChar,    // Character literal
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
    #[must_use]
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
            SpanKind::LitChar => "LitChar",
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
    #[must_use]
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
    #[must_use]
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
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub fn is_module_boundary(&self) -> bool {
        matches!(
            self,
            SpanKind::ImportStatement | SpanKind::ExportStatement | SpanKind::ModuleDecl
        )
    }
}
