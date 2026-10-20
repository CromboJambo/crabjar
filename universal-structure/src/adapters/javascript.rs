//! JavaScript/TypeScript adapter for universal structural token representation.
//!
//! Uses regex-based pattern matching to identify key JS/TS constructs and map them
//! to universal SpanKind variants. This is a proof-of-concept implementation that
//! demonstrates the pluggable adapter architecture without requiring a full JS parser.

use crate::{LanguageAdapter, Span, SpanKind, SpanTree};
use std::collections::HashMap;

/// Simplified JavaScript token kind for internal use.
#[derive(Debug, Clone, PartialEq)]
enum JsToken {
    Keyword(String),
    Identifier(String),
    Punctuation(String),
    StringLiteral(String),
    NumberLiteral(String),
}

/// JavaScript/TypeScript adapter implementing the LanguageAdapter trait.
pub struct JavaScriptAdapter;

impl JavaScriptAdapter {
    pub fn new() -> Self {
        JavaScriptAdapter
    }

    /// Tokenize JavaScript source into simplified tokens using regex patterns.
    fn tokenize(&self, source: &str) -> Vec<JsToken> {
        let mut tokens = Vec::new();
        let chars: Vec<char> = source.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Skip whitespace
            if chars[i].is_whitespace() {
                i += 1;
                continue;
            }

            // String literals (single and double quotes)
            if chars[i] == '"' || chars[i] == '\'' {
                let quote = chars[i];
                i += 1;
                let mut content = String::new();
                while i < chars.len() && chars[i] != quote {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        i += 2;
                    } else {
                        content.push(chars[i]);
                        i += 1;
                    }
                }
                if i < chars.len() {
                    i += 1; // closing quote
                }
                tokens.push(JsToken::StringLiteral(content));
                continue;
            }

            // Number literals
            if chars[i].is_ascii_digit() {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                tokens.push(JsToken::NumberLiteral(source[start..i].to_string()));
                continue;
            }

            // Identifiers and keywords
            if chars[i].is_alphabetic() || chars[i] == '_' || chars[i] == '$' {
                let start = i;
                while i < chars.len()
                    && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '$')
                {
                    i += 1;
                }
                let word = source[start..i].to_string();

                // Check if it's a keyword
                let keywords = [
                    "function",
                    "const",
                    "let",
                    "var",
                    "if",
                    "else",
                    "while",
                    "for",
                    "return",
                    "import",
                    "export",
                    "from",
                    "class",
                    "extends",
                    "new",
                    "try",
                    "catch",
                    "finally",
                    "throw",
                    "async",
                    "await",
                    "switch",
                    "case",
                    "break",
                    "continue",
                    "typeof",
                    "instanceof",
                ];
                if keywords.contains(&word.as_str()) {
                    tokens.push(JsToken::Keyword(word));
                } else {
                    tokens.push(JsToken::Identifier(word));
                }
                continue;
            }

            // Punctuation
            let start = i;
            while i < chars.len() && !chars[i].is_alphanumeric() && !chars[i].is_whitespace() {
                i += 1;
            }
            tokens.push(JsToken::Punctuation(source[start..i].to_string()));
        }

        tokens
    }

    /// Detect async/await patterns in source.
    fn detect_async_patterns(&self, source: &str) -> HashMap<String, String> {
        let mut tags = HashMap::new();

        if source.contains("async ") && source.contains("await ") {
            tags.insert("async_pattern".to_string(), "async_await".to_string());
        } else if source.contains("Promise") {
            tags.insert("async_pattern".to_string(), "promise".to_string());
        } else if source.contains(".then(") {
            tags.insert("async_pattern".to_string(), "then_chain".to_string());
        }

        // Detect callback patterns
        if source.contains("function(") && !source.contains("=>") {
            tags.insert(
                "callback_pattern".to_string(),
                "anonymous_function".to_string(),
            );
        } else if source.contains("=>") {
            tags.insert("callback_pattern".to_string(), "arrow_function".to_string());
        }

        tags
    }

    /// Detect module system patterns.
    fn detect_module_patterns(&self, source: &str) -> HashMap<String, String> {
        let mut tags = HashMap::new();

        if source.contains("import ") || source.contains("export ") {
            tags.insert("module_system".to_string(), "es_modules".to_string());
        } else if source.contains("require(") {
            tags.insert("module_system".to_string(), "commonjs".to_string());
        } else if source.contains("module.exports") {
            tags.insert("module_system".to_string(), "commonjs_exports".to_string());
        }

        tags
    }
}

impl LanguageAdapter for JavaScriptAdapter {
    fn name(&self) -> &str {
        "javascript"
    }

    fn tokenize_and_adapt(&self, source: &str) -> SpanTree {
        let tokens = self.tokenize(source);

        // Build simplified span tree from tokens
        let mut spans = Vec::new();

        for (i, token) in tokens.iter().enumerate() {
            match token {
                JsToken::Keyword(kw) if kw == "function" || kw == "async" => {
                    // Look ahead for function name or arrow
                    if i + 1 < tokens.len() {
                        if let JsToken::Identifier(name) = &tokens[i + 1] {
                            spans.push(Span {
                                kind: SpanKind::FnDecl,
                                open_token: None,
                                body: Vec::new(),
                                close_token: None,
                                source: format!("{} {}", kw, name),
                            });
                        } else if let JsToken::Punctuation(p) = &tokens[i + 1] {
                            if p == "(" {
                                spans.push(Span {
                                    kind: SpanKind::FnDecl,
                                    open_token: None,
                                    body: Vec::new(),
                                    close_token: None,
                                    source: kw.to_string(),
                                });
                            }
                        }
                    }
                }
                JsToken::Keyword(kw) if kw == "class" => {
                    if i + 1 < tokens.len() {
                        if let JsToken::Identifier(name) = &tokens[i + 1] {
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
                JsToken::Keyword(kw) if kw == "const" || kw == "let" => {
                    spans.push(Span {
                        kind: SpanKind::LetStmt,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: kw.to_string(),
                    });
                }
                JsToken::Keyword(kw) if kw == "try" => {
                    spans.push(Span {
                        kind: SpanKind::TryBlock,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: "try".to_string(),
                    });
                }
                JsToken::Keyword(kw) if kw == "catch" => {
                    spans.push(Span {
                        kind: SpanKind::CatchBlock,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: "catch".to_string(),
                    });
                }
                JsToken::Keyword(kw) if kw == "import" || kw == "export" => {
                    spans.push(Span {
                        kind: SpanKind::ImportStatement,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: kw.to_string(),
                    });
                }
                JsToken::StringLiteral(content) => {
                    spans.push(Span {
                        kind: SpanKind::LitStr,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: format!("\"{}\"", content),
                    });
                }
                JsToken::NumberLiteral(content) => {
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

        // Add semantic annotations
        let mut semantic_tags = HashMap::new();
        semantic_tags.extend(self.detect_async_patterns(source));
        semantic_tags.extend(self.detect_module_patterns(source));

        SpanTree {
            spans,
            source: source.to_string(),
            language: "javascript".to_string(),
            semantic_tags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_javascript_adapter_name() {
        let adapter = JavaScriptAdapter::new();
        assert_eq!(adapter.name(), "javascript");
    }

    #[test]
    fn test_tokenize_keywords() {
        let adapter = JavaScriptAdapter::new();
        let tokens = adapter.tokenize("function foo() {}");

        // Should find 'function' keyword and 'foo' identifier
        let has_function = tokens
            .iter()
            .any(|t| matches!(t, JsToken::Keyword(k) if k == "function"));
        let has_foo = tokens
            .iter()
            .any(|t| matches!(t, JsToken::Identifier(i) if i == "foo"));

        assert!(has_function);
        assert!(has_foo);
    }

    #[test]
    fn test_adapt_function_declaration() {
        let adapter = JavaScriptAdapter::new();
        let tree = adapter.tokenize_and_adapt("function processData(input) { return input; }");

        // Should identify a function declaration
        let has_fn_decl = tree.spans.iter().any(|s| s.kind == SpanKind::FnDecl);
        assert!(has_fn_decl);
    }

    #[test]
    fn test_adapt_class_declaration() {
        let adapter = JavaScriptAdapter::new();
        let tree = adapter.tokenize_and_adapt("class MyClass { constructor() {} }");

        // Should identify a class declaration
        let has_class_decl = tree.spans.iter().any(|s| s.kind == SpanKind::ClassDecl);
        assert!(has_class_decl);
    }

    #[test]
    fn test_adapt_async_await() {
        let adapter = JavaScriptAdapter::new();
        let tree = adapter.tokenize_and_adapt("async function fetchData() { await promise; }");

        // Should identify async function and detect async pattern
        let has_fn_decl = tree.spans.iter().any(|s| s.kind == SpanKind::FnDecl);
        assert!(has_fn_decl);

        assert_eq!(
            tree.semantic_tags.get("async_pattern"),
            Some(&"async_await".to_string())
        );
    }

    #[test]
    fn test_adapt_es_modules() {
        let adapter = JavaScriptAdapter::new();
        let tree = adapter.tokenize_and_adapt("import { foo } from './bar'; export const x = 1;");

        // Should detect ES modules
        assert_eq!(
            tree.semantic_tags.get("module_system"),
            Some(&"es_modules".to_string())
        );
    }

    #[test]
    fn test_adapt_try_catch() {
        let adapter = JavaScriptAdapter::new();
        let tree = adapter.tokenize_and_adapt("try { risky(); } catch (e) { handle(e); }");

        // Should identify try and catch blocks
        let has_try = tree.spans.iter().any(|s| s.kind == SpanKind::TryBlock);
        let has_catch = tree.spans.iter().any(|s| s.kind == SpanKind::CatchBlock);

        assert!(has_try);
        assert!(has_catch);
    }

    #[test]
    fn test_adapt_variable_declaration() {
        let adapter = JavaScriptAdapter::new();
        let tree = adapter.tokenize_and_adapt("const x = 42; let y = 'hello';");

        // Should identify variable declarations
        let let_stmts: Vec<_> = tree
            .spans
            .iter()
            .filter(|s| s.kind == SpanKind::LetStmt)
            .collect();
        assert_eq!(let_stmts.len(), 2);
    }

    #[test]
    fn test_adapt_commonjs() {
        let adapter = JavaScriptAdapter::new();
        let tree = adapter.tokenize_and_adapt("const fs = require('fs'); module.exports = {}");

        // Should detect CommonJS
        assert_eq!(
            tree.semantic_tags.get("module_system"),
            Some(&"commonjs".to_string())
        );
    }
}
