//! Python adapter for universal structural token representation.
//!
//! Uses regex-based pattern matching to identify key Python constructs and map them
//! to universal SpanKind variants. This follows the same approach as the JavaScript
//! adapter, demonstrating the pluggable adapter architecture without requiring a full
//! Python parser.

use crate::{LanguageAdapter, Span, SpanKind, SpanTree};
use std::collections::HashMap;

/// Simplified Python token kind for internal use.
#[derive(Debug, Clone, PartialEq)]
enum PyToken {
    Keyword(String),
    Identifier(String),
    Punctuation(String),
    StringLiteral(String),
    NumberLiteral(String),
    Decorator(String),
}

/// Python adapter implementing the LanguageAdapter trait.
pub struct PythonAdapter;

impl PythonAdapter {
    pub fn new() -> Self {
        PythonAdapter
    }

    /// Tokenize Python source into simplified tokens using regex patterns.
    fn tokenize(&self, source: &str) -> Vec<PyToken> {
        let mut tokens = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        for line in lines {
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Detect decorators
            if trimmed.starts_with('@') {
                tokens.push(PyToken::Decorator(trimmed.to_string()));
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

                // String literals (single, double, triple-quoted)
                if ch == '"' || ch == '\'' {
                    let quote = ch;
                    let mut content = String::new();
                    let mut is_triple = false;

                    // Check for triple quotes
                    if chars.peek() == Some(&quote) && chars.clone().nth(1) == Some(quote) {
                        is_triple = true;
                        chars.next();
                        chars.next();
                    }

                    while let Some(c) = chars.next() {
                        if c == quote {
                            if is_triple {
                                // Check for closing triple quote
                                if chars.peek() == Some(&quote) && chars.clone().nth(1) == Some(quote) {
                                    chars.next();
                                    chars.next();
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        content.push(c);
                    }

                    tokens.push(PyToken::StringLiteral(content));
                    continue;
                }

                // Number literals
                if ch.is_ascii_digit() {
                    let mut num = String::new();
                    num.push(ch);
                    while let Some(&c) = chars.peek() {
                        if c.is_ascii_digit() || c == '.' {
                            num.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    tokens.push(PyToken::NumberLiteral(num));
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
                        "def", "class", "import", "from", "as", "if", "elif", "else",
                        "for", "while", "return", "try", "except", "finally", "raise",
                        "with", "yield", "async", "await", "pass", "break", "continue",
                        "in", "not", "and", "or", "is", "lambda", "global", "nonlocal",
                        "del", "assert", "True", "False", "None",
                    ];

                    if keywords.contains(&word.as_str()) {
                        tokens.push(PyToken::Keyword(word));
                    } else {
                        tokens.push(PyToken::Identifier(word));
                    }
                    continue;
                }

                // Punctuation
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
                }
                tokens.push(PyToken::Punctuation(punct));
            }
        }

        tokens
    }

    /// Detect async patterns in source.
    fn detect_async_patterns(&self, source: &str) -> HashMap<String, String> {
        let mut tags = HashMap::new();

        if source.contains("async def") {
            tags.insert("async_pattern".to_string(), "async_def".to_string());
        }
        if source.contains("await ") {
            tags.insert("async_pattern".to_string(), "await".to_string());
        }
        if source.contains("asyncio") {
            tags.insert("async_library".to_string(), "asyncio".to_string());
        }

        tags
    }

    /// Detect type annotation patterns.
    fn detect_type_patterns(&self, source: &str) -> HashMap<String, String> {
        let mut tags = HashMap::new();

        if source.contains(": ") && (source.contains("->") || source.contains(", :")) {
            tags.insert("type_annotations".to_string(), "explicit".to_string());
        }
        if source.contains("from typing import") {
            tags.insert("typing_imports".to_string(), "true".to_string());
        }

        tags
    }

    /// Detect decorator patterns.
    fn detect_decorator_patterns(&self, source: &str) -> HashMap<String, String> {
        let mut tags = HashMap::new();

        if source.contains("@staticmethod") || source.contains("@classmethod") {
            tags.insert("method_decorators".to_string(), "true".to_string());
        }
        if source.contains("@property") {
            tags.insert("property_decorators".to_string(), "true".to_string());
        }

        tags
    }
}

impl LanguageAdapter for PythonAdapter {
    fn name(&self) -> &str {
        "python"
    }

    fn tokenize_and_adapt(&self, source: &str) -> SpanTree {
        let tokens = self.tokenize(source);

        // Build simplified span tree from tokens
        let mut spans = Vec::new();

        for (i, token) in tokens.iter().enumerate() {
            match token {
                PyToken::Keyword(kw) if kw == "def" || kw == "async" => {
                    // Look ahead for function name
                    let is_async = kw == "async";
                    let start_idx = if is_async && i + 1 < tokens.len() {
                        match &tokens[i + 1] {
                            PyToken::Keyword(k) if k == "def" => i + 2,
                            _ => i + 1,
                        }
                    } else {
                        i + 1
                    };

                    if start_idx < tokens.len() {
                        if let PyToken::Identifier(name) = &tokens[start_idx] {
                            spans.push(Span {
                                kind: SpanKind::FnDecl,
                                open_token: None,
                                body: Vec::new(),
                                close_token: None,
                                source: format!("{} def {}", if is_async { "async" } else { "" }, name),
                            });
                        }
                    }
                }
                PyToken::Keyword(kw) if kw == "class" => {
                    if i + 1 < tokens.len() {
                        if let PyToken::Identifier(name) = &tokens[i + 1] {
                            spans.push(Span {
                                kind: SpanKind::ClassDecl,
                                open_token: None,
                                body: Vec::new(),
                                close_token: None,
                                source: format!("class {}", name),
                            });
                        }
                    }
                }
                PyToken::Keyword(kw) if kw == "if" || kw == "elif" => {
                    spans.push(Span {
                        kind: SpanKind::IfStmt,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: kw.to_string(),
                    });
                }
                PyToken::Keyword(kw) if kw == "for" || kw == "while" => {
                    spans.push(Span {
                        kind: SpanKind::ForLoop,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: kw.to_string(),
                    });
                }
                PyToken::Keyword(kw) if kw == "try" => {
                    spans.push(Span {
                        kind: SpanKind::TryBlock,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: "try".to_string(),
                    });
                }
                PyToken::Keyword(kw) if kw == "except" => {
                    spans.push(Span {
                        kind: SpanKind::CatchBlock,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: "except".to_string(),
                    });
                }
                PyToken::Keyword(kw) if kw == "import" || kw == "from" => {
                    spans.push(Span {
                        kind: SpanKind::ImportStatement,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: kw.to_string(),
                    });
                }
                PyToken::StringLiteral(content) => {
                    spans.push(Span {
                        kind: SpanKind::LitStr,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: format!("\"{}\"", content),
                    });
                }
                PyToken::NumberLiteral(content) => {
                    spans.push(Span {
                        kind: SpanKind::LitInt,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: content.clone(),
                    });
                }
                PyToken::Decorator(dec) => {
                    spans.push(Span {
                        kind: SpanKind::FnDecl, // Decorators wrap functions
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: dec.clone(),
                    });
                }
                _ => {}
            }
        }

        // Add semantic annotations
        let mut semantic_tags = HashMap::new();
        semantic_tags.extend(self.detect_async_patterns(source));
        semantic_tags.extend(self.detect_type_patterns(source));
        semantic_tags.extend(self.detect_decorator_patterns(source));

        SpanTree {
            spans,
            source: source.to_string(),
            language: "python".to_string(),
            semantic_tags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_adapter_name() {
        let adapter = PythonAdapter::new();
        assert_eq!(adapter.name(), "python");
    }

    #[test]
    fn test_tokenize_function_declaration() {
        let adapter = PythonAdapter::new();
        let tree = adapter.tokenize_and_adapt("def hello(): pass");

        let has_fn_decl = tree.spans.iter().any(|s| s.kind == SpanKind::FnDecl);
        assert!(has_fn_decl);
    }

    #[test]
    fn test_tokenize_class_declaration() {
        let adapter = PythonAdapter::new();
        let tree = adapter.tokenize_and_adapt("class MyClass: pass");

        let has_class_decl = tree.spans.iter().any(|s| s.kind == SpanKind::ClassDecl);
        assert!(has_class_decl);
    }

    #[test]
    fn test_tokenize_import_statement() {
        let adapter = PythonAdapter::new();
        let tree = adapter.tokenize_and_adapt("import os");

        let has_import = tree.spans.iter().any(|s| s.kind == SpanKind::ImportStatement);
        assert!(has_import);
    }

    #[test]
    fn test_tokenize_try_except() {
        let adapter = PythonAdapter::new();
        let tree = adapter.tokenize_and_adapt("try:\n    pass\nexcept:\n    pass");

        let has_try = tree.spans.iter().any(|s| s.kind == SpanKind::TryBlock);
        let has_catch = tree.spans.iter().any(|s| s.kind == SpanKind::CatchBlock);
        assert!(has_try && has_catch);
    }

    #[test]
    fn test_tokenize_async_function() {
        let adapter = PythonAdapter::new();
        let tree = adapter.tokenize_and_adapt("async def fetch(): pass");

        let has_fn_decl = tree.spans.iter().any(|s| s.kind == SpanKind::FnDecl);
        assert!(has_fn_decl);
        assert!(tree.semantic_tags.get("async_pattern").is_some());
    }

    #[test]
    fn test_tokenize_decorator() {
        let adapter = PythonAdapter::new();
        let tree = adapter.tokenize_and_adapt("@property\ndef name(self): pass");

        // Decorators are mapped to FnDecl spans
        assert!(tree.spans.iter().any(|s| s.source.starts_with("@")));
    }

    #[test]
    fn test_tokenize_string_literal() {
        let adapter = PythonAdapter::new();
        let tree = adapter.tokenize_and_adapt("x = \"hello\"");

        let has_str = tree.spans.iter().any(|s| s.kind == SpanKind::LitStr);
        assert!(has_str);
    }

    #[test]
    fn test_tokenize_number_literal() {
        let adapter = PythonAdapter::new();
        let tree = adapter.tokenize_and_adapt("x = 42");

        let has_num = tree.spans.iter().any(|s| s.kind == SpanKind::LitInt);
        assert!(has_num);
    }

    #[test]
    fn test_cross_language_compatibility() {
        let py_adapter = PythonAdapter::new();
        let js_adapter = crate::adapters::JavaScriptAdapter::new();

        // Both should detect functions
        let py_tree = py_adapter.tokenize_and_adapt("def hello(): pass");
        let js_tree = js_adapter.tokenize_and_adapt("function hello() {}");

        assert!(py_tree.spans.iter().any(|s| s.kind == SpanKind::FnDecl));
        assert!(js_tree.spans.iter().any(|s| s.kind == SpanKind::FnDecl));
    }
}