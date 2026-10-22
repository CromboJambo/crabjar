//! Go adapter for universal structural token representation.
//!
//! Uses regex-based pattern matching to identify key Go constructs and map them
//! to universal SpanKind variants. This follows the same approach as the JavaScript,
//! Python, C, and C++ adapters, demonstrating the pluggable adapter architecture without
//! requiring a full Go parser.

use crate::{LanguageAdapter, Span, SpanKind, SpanTree};
use std::collections::HashMap;

/// Simplified Go token kind for internal use.
#[derive(Debug, Clone, PartialEq)]
enum GoTok {
    Keyword(String),
    Identifier(String),
    Punctuation(String),
    StringLiteral(String),
    CharLiteral(char),
    NumberLiteral(String),
    Comment(String),
}

/// Go adapter implementing the LanguageAdapter trait.
pub struct GoAdapter;

impl GoAdapter {
    pub fn new() -> Self {
        GoAdapter
    }

    /// Tokenize Go source into simplified tokens using regex patterns.
    fn tokenize(&self, source: &str) -> Vec<GoTok> {
        let mut tokens = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        for line in lines {
            let trimmed = line.trim();

            // Skip empty lines
            if trimmed.is_empty() {
                continue;
            }

            // Detect comments
            if trimmed.starts_with("//") {
                tokens.push(GoTok::Comment(trimmed.to_string()));
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

                // String literals (double quotes)
                if ch == '"' {
                    let mut content = String::new();
                    while let Some(c) = chars.next() {
                        if c == '\\' {
                            if let Some(esc) = chars.next() {
                                content.push(esc);
                            }
                        } else if c == '"' {
                            break;
                        } else {
                            content.push(c);
                        }
                    }
                    tokens.push(GoTok::StringLiteral(content));
                    continue;
                }

                // Character literals (single quotes)
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
                        tokens.push(GoTok::CharLiteral(content.chars().next().unwrap()));
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
                    tokens.push(GoTok::NumberLiteral(num));
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

                    // Go keywords
                    let keywords = [
                        "break", "case", "chan", "const", "continue", "default", "defer", "else",
                        "fallthrough", "for", "func", "go", "goto", "if", "import", "interface",
                        "map", "package", "range", "return", "select", "struct", "switch", "type",
                        "var",
                    ];

                    if keywords.contains(&word.as_str()) {
                        tokens.push(GoTok::Keyword(word));
                    } else {
                        tokens.push(GoTok::Identifier(word));
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
                }
                tokens.push(GoTok::Punctuation(punct));
            }
        }

        tokens
    }

    /// Detect goroutine patterns.
    fn detect_goroutine_patterns(&self, source: &str) -> HashMap<String, String> {
        let mut tags = HashMap::new();

        if source.contains("go ") && (source.contains("(") || source.contains(".")) {
            tags.insert("goroutines".to_string(), "true".to_string());
        }

        tags
    }

    /// Detect channel patterns.
    fn detect_channel_patterns(&self, source: &str) -> HashMap<String, String> {
        let mut tags = HashMap::new();

        if source.contains("chan ") || source.contains("<-") {
            tags.insert("channels".to_string(), "true".to_string());
        }

        tags
    }

    /// Detect error handling patterns.
    fn detect_error_patterns(&self, source: &str) -> HashMap<String, String> {
        let mut tags = HashMap::new();

        if source.contains("defer ") {
            tags.insert("defer_used".to_string(), "true".to_string());
        }
        if source.contains("recover()") || source.contains("panic(") {
            tags.insert("panic_recover".to_string(), "true".to_string());
        }

        tags
    }
}

impl LanguageAdapter for GoAdapter {
    fn name(&self) -> &str {
        "go"
    }

    fn tokenize_and_adapt(&self, source: &str) -> SpanTree {
        let tokens = self.tokenize(source);

        // Build simplified span tree from tokens
        let mut spans = Vec::new();

        for (i, token) in tokens.iter().enumerate() {
            match token {
                GoTok::Keyword(kw) if kw == "package" => {
                    // Look ahead for package name
                    if i + 1 < tokens.len() {
                        if let GoTok::Identifier(name) = &tokens[i + 1] {
                            spans.push(Span {
                                kind: SpanKind::ModuleDecl,
                                open_token: None,
                                body: Vec::new(),
                                close_token: None,
                                source: format!("package {}", name),
                            });
                        }
                    }
                }
                GoTok::Keyword(kw) if kw == "import" => {
                    spans.push(Span {
                        kind: SpanKind::ImportStatement,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: "import".to_string(),
                    });
                }
                GoTok::Keyword(kw) if kw == "func" => {
                    // Look ahead for function name
                    if i + 1 < tokens.len() {
                        if let GoTok::Identifier(name) = &tokens[i + 1] {
                            spans.push(Span {
                                kind: SpanKind::FnDecl,
                                open_token: None,
                                body: Vec::new(),
                                close_token: None,
                                source: format!("func {}", name),
                            });
                        }
                    }
                }
                GoTok::Keyword(kw) if kw == "type" => {
                    spans.push(Span {
                        kind: SpanKind::ClassDecl,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: "type".to_string(),
                    });
                }
                GoTok::Keyword(kw) if kw == "struct" => {
                    spans.push(Span {
                        kind: SpanKind::ClassDecl,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: "struct".to_string(),
                    });
                }
                GoTok::Keyword(kw) if kw == "interface" => {
                    spans.push(Span {
                        kind: SpanKind::TraitItem,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: "interface".to_string(),
                    });
                }
                GoTok::Keyword(kw) if kw == "var" => {
                    spans.push(Span {
                        kind: SpanKind::LetStmt,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: "var".to_string(),
                    });
                }
                GoTok::Keyword(kw) if kw == "const" => {
                    spans.push(Span {
                        kind: SpanKind::ConstStmt,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: "const".to_string(),
                    });
                }
                GoTok::Keyword(kw) if kw == "if" || kw == "else" => {
                    spans.push(Span {
                        kind: SpanKind::IfStmt,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: kw.to_string(),
                    });
                }
                GoTok::Keyword(kw) if kw == "for" || kw == "range" => {
                    spans.push(Span {
                        kind: SpanKind::ForLoop,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: kw.to_string(),
                    });
                }
                GoTok::Keyword(kw) if kw == "switch" => {
                    spans.push(Span {
                        kind: SpanKind::Block,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: "switch".to_string(),
                    });
                }
                GoTok::Keyword(kw) if kw == "case" => {
                    spans.push(Span {
                        kind: SpanKind::Pattern,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: "case".to_string(),
                    });
                }
                GoTok::Keyword(kw) if kw == "select" => {
                    spans.push(Span {
                        kind: SpanKind::Block,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: "select".to_string(),
                    });
                }
                GoTok::Keyword(kw) if kw == "defer" => {
                    spans.push(Span {
                        kind: SpanKind::Stmt,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: "defer".to_string(),
                    });
                }
                GoTok::Keyword(kw) if kw == "go" => {
                    spans.push(Span {
                        kind: SpanKind::Stmt,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: "go".to_string(),
                    });
                }
                GoTok::StringLiteral(content) => {
                    spans.push(Span {
                        kind: SpanKind::LitStr,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: format!("\"{}\"", content),
                    });
                }
                GoTok::CharLiteral(ch) => {
                    spans.push(Span {
                        kind: SpanKind::LitChar,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: format!("'{}'", ch),
                    });
                }
                GoTok::NumberLiteral(content) => {
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
        semantic_tags.extend(self.detect_goroutine_patterns(source));
        semantic_tags.extend(self.detect_channel_patterns(source));
        semantic_tags.extend(self.detect_error_patterns(source));

        SpanTree {
            spans,
            source: source.to_string(),
            language: "go".to_string(),
            semantic_tags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_go_adapter_name() {
        let adapter = GoAdapter::new();
        assert_eq!(adapter.name(), "go");
    }

    #[test]
    fn test_tokenize_package_declaration() {
        let adapter = GoAdapter::new();
        let tree = adapter.tokenize_and_adapt("package main");

        let has_module = tree.spans.iter().any(|s| s.kind == SpanKind::ModuleDecl);
        assert!(has_module);
    }

    #[test]
    fn test_tokenize_import_statement() {
        let adapter = GoAdapter::new();
        let tree = adapter.tokenize_and_adapt("import \"fmt\"");

        let has_import = tree.spans.iter().any(|s| s.kind == SpanKind::ImportStatement);
        assert!(has_import);
    }

    #[test]
    fn test_tokenize_function_definition() {
        let adapter = GoAdapter::new();
        let tree = adapter.tokenize_and_adapt("func main() {}");

        let has_func = tree.spans.iter().any(|s| s.kind == SpanKind::FnDecl);
        assert!(has_func);
    }

    #[test]
    fn test_tokenize_struct_definition() {
        let adapter = GoAdapter::new();
        let tree = adapter.tokenize_and_adapt("type Person struct { Name string }");

        let has_type = tree.spans.iter().any(|s| s.kind == SpanKind::ClassDecl);
        assert!(has_type);
    }

    #[test]
    fn test_tokenize_interface_definition() {
        let adapter = GoAdapter::new();
        let tree = adapter.tokenize_and_adapt("type Reader interface { Read([]byte) (int, error) }");

        let has_interface = tree.spans.iter().any(|s| s.kind == SpanKind::TraitItem);
        assert!(has_interface);
    }

    #[test]
    fn test_tokenize_control_flow() {
        let adapter = GoAdapter::new();
        let tree = adapter.tokenize_and_adapt("if x > 0 { return true } else { return false }");

        let has_if = tree.spans.iter().any(|s| s.kind == SpanKind::IfStmt);
        assert!(has_if);
    }

    #[test]
    fn test_tokenize_for_loop() {
        let adapter = GoAdapter::new();
        let tree = adapter.tokenize_and_adapt("for i := 0; i < 10; i++ {}");

        let has_for = tree.spans.iter().any(|s| s.kind == SpanKind::ForLoop);
        assert!(has_for);
    }

    #[test]
    fn test_tokenize_range_loop() {
        let adapter = GoAdapter::new();
        let tree = adapter.tokenize_and_adapt("for _, item := range items {}");

        let has_range = tree.spans.iter().any(|s| s.kind == SpanKind::ForLoop);
        assert!(has_range);
    }

    #[test]
    fn test_cross_language_compatibility() {
        let go_adapter = GoAdapter::new();
        let js_adapter = crate::adapters::JavaScriptAdapter::new();

        // Both should detect functions
        let go_tree = go_adapter.tokenize_and_adapt("func hello() {}");
        let js_tree = js_adapter.tokenize_and_adapt("function hello() {}");

        assert!(go_tree.spans.iter().any(|s| s.kind == SpanKind::FnDecl));
        assert!(js_tree.spans.iter().any(|s| s.kind == SpanKind::FnDecl));
    }
}