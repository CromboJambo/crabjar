//! C adapter for universal structural token representation.
//!
//! Uses regex-based pattern matching to identify key C constructs and map them
//! to universal SpanKind variants. This follows the same approach as the JavaScript
//! and Python adapters, demonstrating the pluggable adapter architecture without
//! requiring a full C parser.

use crate::{LanguageAdapter, Span, SpanKind, SpanTree};
use std::collections::HashMap;

/// Simplified C token kind for internal use.
#[derive(Debug, Clone, PartialEq)]
enum CTok {
    Keyword(String),
    Identifier(String),
    Punctuation(String),
    StringLiteral(String),
    CharLiteral(char),
    NumberLiteral(String),
    Preprocessor(String),
}

/// C adapter implementing the LanguageAdapter trait.
pub struct CAdapter;

impl CAdapter {
    pub fn new() -> Self {
        CAdapter
    }

    /// Tokenize C source into simplified tokens using regex patterns.
    fn tokenize(&self, source: &str) -> Vec<CTok> {
        let mut tokens = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        for line in lines {
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") {
                continue;
            }

            // Detect preprocessor directives
            if trimmed.starts_with('#') {
                tokens.push(CTok::Preprocessor(trimmed.to_string()));
                continue;
            }

            // Tokenize the line
            let mut chars = trimmed.chars().peekable();
            while chars.peek().is_some() {
                let ch = chars.next().unwrap();

                // Skip whitespace
                if ch.is_whitespace() {
                    continue;
                }

                // String literals
                if ch == '"' {
                    let mut content = String::new();
                    while let Some(c) = chars.next() {
                        if c == '\\' {
                            // Escape sequence
                            if let Some(esc) = chars.next() {
                                content.push(esc);
                            }
                        } else if c == '"' {
                            break;
                        } else {
                            content.push(c);
                        }
                    }
                    tokens.push(CTok::StringLiteral(content));
                    continue;
                }

                // Character literals
                if ch == '\'' {
                    let mut content = String::new();
                    while let Some(c) = chars.next() {
                        if c == '\\' {
                            if let Some(esc) = chars.next() {
                                content.push(esc);
                            }
                        } else if c == '\'' {
                            break;
                        } else {
                            content.push(c);
                        }
                    }
                    if !content.is_empty() {
                        tokens.push(CTok::CharLiteral(content.chars().next().unwrap()));
                    }
                    continue;
                }

                // Number literals
                if ch.is_ascii_digit() || (ch == '.' && chars.peek().is_some() && chars.peek().unwrap().is_ascii_digit()) {
                    let mut num = String::new();
                    num.push(ch);
                    while let Some(&c) = chars.peek() {
                        if c.is_ascii_alphanumeric() || c == '.' || c == 'x' || c == 'X' {
                            num.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    tokens.push(CTok::NumberLiteral(num));
                    continue;
                }

                // Identifiers and keywords
                if ch.is_alphabetic() || ch == '_' {
                    let mut word = String::new();
                    word.push(ch);
                    while let Some(&c) = chars.peek() {
                        if c.is_alphanumeric() || c == '_' {
                            word.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }

                    // Check if it's a keyword
                    let keywords = [
                        "auto", "break", "case", "char", "const", "continue", "default", "do",
                        "double", "else", "enum", "extern", "float", "for", "goto", "if",
                        "inline", "int", "long", "register", "restrict", "return", "short",
                        "signed", "sizeof", "static", "struct", "switch", "typedef", "union",
                        "unsigned", "void", "volatile", "while", "_Bool", "_Complex", "_Imaginary",
                    ];

                    if keywords.contains(&word.as_str()) {
                        tokens.push(CTok::Keyword(word));
                    } else {
                        tokens.push(CTok::Identifier(word));
                    }
                    continue;
                }

                // Punctuation and operators
                let mut punct = String::new();
                punct.push(ch);
                // Handle multi-char operators
                if ch == '=' && chars.peek() == Some(&'=') {
                    punct.push(chars.next().unwrap());
                } else if ch == '!' && chars.peek() == Some(&'=') {
                    punct.push(chars.next().unwrap());
                } else if ch == '<' && (chars.peek() == Some(&'=') || chars.peek() == Some(&'<')) {
                    punct.push(chars.next().unwrap());
                } else if ch == '>' && (chars.peek() == Some(&'=') || chars.peek() == Some(&'>')) {
                    punct.push(chars.next().unwrap());
                } else if ch == '&' && chars.peek() == Some(&'&') {
                    punct.push(chars.next().unwrap());
                } else if ch == '|' && chars.peek() == Some(&'|') {
                    punct.push(chars.next().unwrap());
                } else if ch == '+' && chars.peek() == Some(&'+') {
                    punct.push(chars.next().unwrap());
                } else if ch == '-' && chars.peek() == Some(&'-') {
                    punct.push(chars.next().unwrap());
                } else if ch == '*' && chars.peek() == Some(&'=') {
                    punct.push(chars.next().unwrap());
                } else if ch == '/' && chars.peek() == Some(&'=') {
                    punct.push(chars.next().unwrap());
                }
                tokens.push(CTok::Punctuation(punct));
            }
        }

        tokens
    }

    /// Detect pointer patterns.
    fn detect_pointer_patterns(&self, source: &str) -> HashMap<String, String> {
        let mut tags = HashMap::new();

        if source.contains("*") {
            tags.insert("pointer_usage".to_string(), "true".to_string());
        }
        if source.contains("**") || source.contains("(*") {
            tags.insert("pointer_to_pointer".to_string(), "true".to_string());
        }

        tags
    }

    /// Detect memory management patterns.
    fn detect_memory_patterns(&self, source: &str) -> HashMap<String, String> {
        let mut tags = HashMap::new();

        if source.contains("malloc") || source.contains("calloc") || source.contains("realloc") {
            tags.insert("dynamic_allocation".to_string(), "true".to_string());
        }
        if source.contains("free") {
            tags.insert("manual_deallocation".to_string(), "true".to_string());
        }

        tags
    }

    /// Detect preprocessor patterns.
    fn detect_preprocessor_patterns(&self, source: &str) -> HashMap<String, String> {
        let mut tags = HashMap::new();

        if source.contains("#ifdef") || source.contains("#ifndef") {
            tags.insert("conditional_compilation".to_string(), "true".to_string());
        }
        if source.contains("#include") {
            tags.insert("has_includes".to_string(), "true".to_string());
        }
        if source.contains("#define") {
            tags.insert("has_macros".to_string(), "true".to_string());
        }

        tags
    }
}

impl LanguageAdapter for CAdapter {
    fn name(&self) -> &str {
        "c"
    }

    fn tokenize_and_adapt(&self, source: &str) -> SpanTree {
        let tokens = self.tokenize(source);

        // Build simplified span tree from tokens
        let mut spans = Vec::new();

        for (i, token) in tokens.iter().enumerate() {
            match token {
                CTok::Keyword(kw) if kw == "struct" || kw == "enum" || kw == "union" => {
                    // Look ahead for name
                    if i + 1 < tokens.len() {
                        if let CTok::Identifier(name) = &tokens[i + 1] {
                            spans.push(Span {
                                kind: SpanKind::ClassDecl,
                                open_token: None,
                                body: Vec::new(),
                                close_token: None,
                                source: format!("{} {}", kw, name),
                            });
                        }
                    }
                }
                CTok::Keyword(kw) if kw == "typedef" => {
                    spans.push(Span {
                        kind: SpanKind::LetStmt,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: "typedef".to_string(),
                    });
                }
                CTok::Keyword(kw) if kw == "if" || kw == "else" => {
                    spans.push(Span {
                        kind: SpanKind::IfStmt,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: kw.to_string(),
                    });
                }
                CTok::Keyword(kw) if kw == "for" || kw == "while" => {
                    spans.push(Span {
                        kind: SpanKind::ForLoop,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: kw.to_string(),
                    });
                }
                CTok::Keyword(kw) if kw == "switch" => {
                    spans.push(Span {
                        kind: SpanKind::Block,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: "switch".to_string(),
                    });
                }
                CTok::Keyword(kw) if kw == "case" => {
                    spans.push(Span {
                        kind: SpanKind::Pattern,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: "case".to_string(),
                    });
                }
                CTok::Preprocessor(dir) if dir.starts_with("#include") => {
                    spans.push(Span {
                        kind: SpanKind::ImportStatement,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: dir.clone(),
                    });
                }
                CTok::Preprocessor(dir) if dir.starts_with("#define") => {
                    spans.push(Span {
                        kind: SpanKind::ConstStmt,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: dir.clone(),
                    });
                }
                CTok::StringLiteral(content) => {
                    spans.push(Span {
                        kind: SpanKind::LitStr,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: format!("\"{}\"", content),
                    });
                }
                CTok::CharLiteral(ch) => {
                    spans.push(Span {
                        kind: SpanKind::LitChar,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: format!("'{}'", ch),
                    });
                }
                CTok::NumberLiteral(content) => {
                    spans.push(Span {
                        kind: SpanKind::LitInt,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: content.clone(),
                    });
                }
                _ => {}
            }
        }

        // Detect function definitions by looking for return type + name + ( pattern
        let mut i = 0;
        while i < tokens.len() {
            if let CTok::Identifier(name) = &tokens[i] {
                // Check if next token is '(' — potential function definition
                if i + 1 < tokens.len() {
                    if let CTok::Punctuation(p) = &tokens[i + 1] {
                        if p == "(" {
                            // Look back for return type (identifier or keyword)
                            let has_return_type = i > 0 && matches!(
                                &tokens[i - 1],
                                CTok::Keyword(_) | CTok::Identifier(_)
                            );

                            if has_return_type {
                                spans.push(Span {
                                    kind: SpanKind::FnDecl,
                                    open_token: None,
                                    body: Vec::new(),
                                    close_token: None,
                                    source: name.clone(),
                                });
                            }
                        }
                    }
                }
            }
            i += 1;
        }

        // Add semantic annotations
        let mut semantic_tags = HashMap::new();
        semantic_tags.extend(self.detect_pointer_patterns(source));
        semantic_tags.extend(self.detect_memory_patterns(source));
        semantic_tags.extend(self.detect_preprocessor_patterns(source));

        SpanTree {
            spans,
            source: source.to_string(),
            language: "c".to_string(),
            semantic_tags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let has_import = tree.spans.iter().any(|s| s.kind == SpanKind::ImportStatement);
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
        let rust_adapter = crate::adapters::RustAdapter::new();

        // Both should detect functions
        let c_tree = c_adapter.tokenize_and_adapt("int hello() { return 0; }");
        let rust_tree = rust_adapter.tokenize_and_adapt("fn hello() -> i32 { 42 }");

        assert!(c_tree.spans.iter().any(|s| s.kind == SpanKind::FnDecl));
        assert!(rust_tree.spans.iter().any(|s| s.kind == SpanKind::FnDecl));
    }
}