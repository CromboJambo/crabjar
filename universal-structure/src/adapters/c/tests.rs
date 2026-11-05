//! Tests for the C adapter.

use super::super::CAdapter;
use crate::adapters::RustAdapter;
use crate::{LanguageAdapter, SpanKind};

#[test]
fn test_c_adapter_name() {
    let adapter = CAdapter::new();
    assert_eq!(adapter.name(), "c");
}

#[test]
fn test_tokenize_function_declaration() {
    let adapter = CAdapter::new();
    let tree = adapter.tokenize_and_adapt("int hello() { return 0; }");

    let has_fn_decl = tree.spans.iter().any(|s| s.kind == SpanKind::FnDecl);
    assert!(has_fn_decl);
}

#[test]
fn test_tokenize_struct_declaration() {
    let adapter = CAdapter::new();
    let tree = adapter.tokenize_and_adapt("struct Point { int x; int y; };");

    let has_class_decl = tree.spans.iter().any(|s| s.kind == SpanKind::ClassDecl);
    assert!(has_class_decl);
}

#[test]
fn test_tokenize_include_directive() {
    let adapter = CAdapter::new();
    let tree = adapter.tokenize_and_adapt("#include <stdio.h>");

    let has_import = tree
        .spans
        .iter()
        .any(|s| s.kind == SpanKind::ImportStatement);
    assert!(has_import);
}

#[test]
fn test_tokenize_define_directive() {
    let adapter = CAdapter::new();
    let tree = adapter.tokenize_and_adapt("#define MAX_SIZE 1024");

    let has_const = tree.spans.iter().any(|s| s.kind == SpanKind::ConstStmt);
    assert!(has_const);
}

#[test]
fn test_tokenize_if_statement() {
    let adapter = CAdapter::new();
    let tree = adapter.tokenize_and_adapt("if (x > 0) { y = 1; } else { y = 0; }");

    let has_if = tree.spans.iter().any(|s| s.kind == SpanKind::IfStmt);
    assert!(has_if);
}

#[test]
fn test_tokenize_for_loop() {
    let adapter = CAdapter::new();
    let tree = adapter.tokenize_and_adapt("for (int i = 0; i < 10; i++) { }");

    let has_for = tree.spans.iter().any(|s| s.kind == SpanKind::ForLoop);
    assert!(has_for);
}

#[test]
fn test_tokenize_string_literal() {
    let adapter = CAdapter::new();
    let tree = adapter.tokenize_and_adapt("char *s = \"hello\";");

    let has_str = tree.spans.iter().any(|s| s.kind == SpanKind::LitStr);
    assert!(has_str);
}

#[test]
fn test_tokenize_char_literal() {
    let adapter = CAdapter::new();
    let tree = adapter.tokenize_and_adapt("char c = 'A';");

    let has_char = tree.spans.iter().any(|s| s.kind == SpanKind::LitChar);
    assert!(has_char);
}

#[test]
fn test_cross_language_compatibility() {
    let c_adapter = CAdapter::new();
    let rust_adapter = RustAdapter::new();

    // Both should detect functions
    let c_tree = c_adapter.tokenize_and_adapt("int hello() { return 0; }");
    let rust_tree = rust_adapter.tokenize_and_adapt("fn hello() -> i32 { 42 }");

    assert!(c_tree.spans.iter().any(|s| s.kind == SpanKind::FnDecl));
    assert!(rust_tree.spans.iter().any(|s| s.kind == SpanKind::FnDecl));
}
