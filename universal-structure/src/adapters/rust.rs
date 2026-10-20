//! Rust adapter: maps pesti-structural-tokenizer output to universal tokens.
//!
//! This demonstrates the language-specific input layer for Rust, showing how
//! pesti's existing SpanKind enum maps to the universal representation while
//! preserving language-specific tags.

use crate::{SpanKind, StructuralToken};

/// Rust-specific span kinds from pesti-structural-tokenizer (simulated).
/// In production, this would be imported directly from the pesti crate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PestSpanKind {
    FnDecl,
    StructDecl,
    EnumDecl,
    TraitDecl,
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
}

/// Map a pesti Rust span kind to the universal representation.
pub fn map_to_universal(pest_kind: PestSpanKind) -> SpanKind {
    match pest_kind {
        PestSpanKind::FnDecl => SpanKind::FnDecl,
        PestSpanKind::StructDecl | PestSpanKind::EnumDecl => SpanKind::ClassDecl,
        PestSpanKind::TraitDecl => SpanKind::ClassDecl,
        PestSpanKind::ImplBlock => SpanKind::Block,
        PestSpanKind::LetStmt => SpanKind::LetStmt,
        PestSpanKind::CallExpr => SpanKind::CallExpr,
        PestSpanKind::BinaryExpr => SpanKind::BinaryExpr,
        PestSpanKind::UnaryExpr => SpanKind::UnaryExpr,
        PestSpanKind::LitInt => SpanKind::LitInt,
        PestSpanKind::LitFloat => SpanKind::LitFloat,
        PestSpanKind::LitStr => SpanKind::LitStr,
        PestSpanKind::LitBool => SpanKind::LitBool,
        PestSpanKind::Ident => SpanKind::Ident,
        PestSpanKind::Keyword => SpanKind::Keyword,
        PestSpanKind::Punct => SpanKind::Punct,
        PestSpanKind::Block => SpanKind::Block,
        PestSpanKind::Item => SpanKind::Item,
        PestSpanKind::Stmt => SpanKind::Stmt,
        PestSpanKind::Expr => SpanKind::Expr,
        PestSpanKind::Pattern => SpanKind::Pattern,
        PestSpanKind::Type => SpanKind::Type,
        PestSpanKind::GenericParam => SpanKind::GenericParam,
        PestSpanKind::TraitItem => SpanKind::TraitItem,
    }
}

/// Convert a pesti token to a universal structural token with language tags.
pub fn convert_token(pest_kind: PestSpanKind, text: &str, range: [usize; 2]) -> StructuralToken {
    let kind = map_to_universal(pest_kind);

    // Preserve Rust-specific information in tags
    let mut token = StructuralToken::new(kind, text, range)
        .with_tag("language", "rust")
        .with_tag("original_kind", pesti_kind_name(&pest_kind));

    // Add access level tag for declarations
    if matches!(pest_kind, PestSpanKind::FnDecl | PestSpanKind::StructDecl) {
        token = token.with_tag("access", infer_access_level(text));
    }

    token
}

/// Infer Rust-specific access level from source text.
fn infer_access_level(text: &str) -> &'static str {
    if text.contains("pub(crate)") {
        "pub_crate"
    } else if text.contains("pub ") {
        "public"
    } else {
        "private"
    }
}

/// Get the name of a pesti span kind for tag preservation.
fn pesti_kind_name(kind: &PestSpanKind) -> String {
    match kind {
        PestSpanKind::FnDecl => "FnDecl".to_string(),
        PestSpanKind::StructDecl => "StructDecl".to_string(),
        PestSpanKind::EnumDecl => "EnumDecl".to_string(),
        PestSpanKind::TraitDecl => "TraitDecl".to_string(),
        _ => format!("{:?}", kind),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fn_decl_mapping() {
        let token = convert_token(
            PestSpanKind::FnDecl,
            "pub fn process_data(input: &[u8]) -> Result<String, Error>",
            [0, 60],
        );

        assert_eq!(token.kind, SpanKind::FnDecl);
        assert_eq!(token.tags.get("language").unwrap(), &"rust");
        assert_eq!(token.tags.get("original_kind").unwrap(), &"FnDecl");
        assert_eq!(token.tags.get("access").unwrap(), &"public");
    }

    #[test]
    fn test_struct_decl_mapping() {
        let token = convert_token(PestSpanKind::StructDecl, "pub struct Processor", [0, 20]);

        // Struct maps to ClassDecl in universal representation
        assert_eq!(token.kind, SpanKind::ClassDecl);
        assert_eq!(token.tags.get("original_kind").unwrap(), &"StructDecl");
    }

    #[test]
    fn test_enum_decl_mapping() {
        let token = convert_token(PestSpanKind::EnumDecl, "enum Status", [0, 12]);

        // Enum also maps to ClassDecl (type declaration)
        assert_eq!(token.kind, SpanKind::ClassDecl);
        assert_eq!(token.tags.get("original_kind").unwrap(), &"EnumDecl");
    }

    #[test]
    fn test_private_access_inference() {
        let token = convert_token(PestSpanKind::FnDecl, "fn private_helper()", [0, 20]);
        assert_eq!(token.tags.get("access").unwrap(), &"private");
    }

    #[test]
    fn test_pub_crate_access_inference() {
        let token = convert_token(
            PestSpanKind::FnDecl,
            "pub(crate) fn internal_api()",
            [0, 28],
        );
        assert_eq!(token.tags.get("access").unwrap(), &"pub_crate");
    }

    #[test]
    fn test_expression_mapping() {
        let call = convert_token(PestSpanKind::CallExpr, "process(data)", [0, 14]);
        assert_eq!(call.kind, SpanKind::CallExpr);

        let binary = convert_token(PestSpanKind::BinaryExpr, "a + b", [0, 5]);
        assert_eq!(binary.kind, SpanKind::BinaryExpr);
    }
}
