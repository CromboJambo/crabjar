//! C++ adapter for universal structural token representation.
//!
//! Uses regex-based pattern matching to identify key C++ constructs and map them
//! to universal SpanKind variants. This follows the same approach as the JavaScript,
//! Python, and C adapters, demonstrating the pluggable adapter architecture without
//! requiring a full C++ parser.

use crate::{LanguageAdapter, Span, SpanKind, SpanTree};
use std::collections::HashMap;

/// Simplified C++ token kind for internal use.
#[derive(Debug, Clone, PartialEq)]
enum CppTok {
    Keyword(String),
    Identifier(String),
    Punctuation(String),
    StringLiteral(String),
    CharLiteral(char),
    NumberLiteral(String),
    Preprocessor(String),
}

/// C++ adapter implementing the LanguageAdapter trait.
pub struct CppAdapter;

impl CppAdapter {
    pub fn new() -> Self {
        CppAdapter
    }

    /// Tokenize C++ source into simplified tokens using regex patterns.
    fn tokenize(&self, source: &str) -> Vec<CppTok> {
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
                tokens.push(CppTok::Preprocessor(trimmed.to_string()));
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
                            if let Some(esc) = chars.next() {
                                content.push(esc);
                            }
                        } else if c == '"' {
                            break;
                        } else {
                            content.push(c);
                        }
                    }
                    tokens.push(CppTok::StringLiteral(content));
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
                        tokens.push(CppTok::CharLiteral(content.chars().next().unwrap()));
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
                    tokens.push(CppTok::NumberLiteral(num));
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

                    // Check if it's a keyword (C++ keywords include C keywords plus more)
                    let keywords = [
                        "alignas", "alignof", "and", "and_eq", "asm", "auto", "bitand", "bitor",
                        "bool", "break", "case", "catch", "char", "char8_t", "char16_t", "char32_t",
                        "class", "compl", "concept", "const", "consteval", "constexpr", "constinit",
                        "const_cast", "continue", "co_await", "co_return", "co_yield", "decltype",
                        "default", "delete", "do", "double", "dynamic_cast", "else", "enum",
                        "explicit", "export", "extern", "false", "float", "for", "friend",
                        "goto", "if", "import", "inline", "int", "long", "module", "mutable",
                        "namespace", "new", "noexcept", "not", "not_eq", "nullptr", "operator",
                        "or", "or_eq", "private", "protected", "public", "register", "reinterpret_cast",
                        "requires", "return", "short", "signed", "sizeof", "static", "static_assert",
                        "static_cast", "struct", "switch", "template", "this", "thread_local",
                        "throw", "true", "try", "typedef", "typeid", "typename", "union", "unsigned",
                        "using", "virtual", "void", "volatile", "while", "xor", "xor_eq",
                    ];

                    if keywords.contains(&word.as_str()) {
                        tokens.push(CppTok::Keyword(word));
                    } else {
                        tokens.push(CppTok::Identifier(word));
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
                tokens.push(CppTok::Punctuation(punct));
            }
        }

        tokens
    }

    /// Detect template patterns.
    fn detect_template_patterns(&self, source: &str) -> HashMap<String, String> {
        let mut tags = HashMap::new();

        if source.contains("<") && source.contains(">") {
            tags.insert("template_usage".to_string(), "true".to_string());
        }
        if source.contains("template <") {
            tags.insert("template_declaration".to_string(), "true".to_string());
        }

        tags
    }

    /// Detect smart pointer patterns.
    fn detect_smart_pointer_patterns(&self, source: &str) -> HashMap<String, String> {
        let mut tags = HashMap::new();

        if source.contains("std::unique_ptr") || source.contains("make_unique") {
            tags.insert("smart_pointer".to_string(), "unique_ptr".to_string());
        } else if source.contains("std::shared_ptr") || source.contains("make_shared") {
            tags.insert("smart_pointer".to_string(), "shared_ptr".to_string());
        }

        tags
    }

    /// Detect modern C++ features.
    fn detect_modern_cpp_patterns(&self, source: &str) -> HashMap<String, String> {
        let mut tags = HashMap::new();

        if source.contains("auto") && (source.contains("=") || source.contains("(")) {
            tags.insert("type_inference".to_string(), "true".to_string());
        }
        if source.contains("nullptr") {
            tags.insert("modern_null".to_string(), "nullptr".to_string());
        }
        if source.contains("override") {
            tags.insert("virtual_override".to_string(), "true".to_string());
        }
        if source.contains("final") {
            tags.insert("class_final".to_string(), "true".to_string());
        }

        tags
    }
}

impl LanguageAdapter for CppAdapter {
    fn name(&self) -> &str {
        "cpp"
    }

    fn tokenize_and_adapt(&self, source: &str) -> SpanTree {
        let tokens = self.tokenize(source);

        // Build simplified span tree from tokens
        let mut spans = Vec::new();

        for (i, token) in tokens.iter().enumerate() {
            match token {
                CppTok::Keyword(kw) if kw == "class" || kw == "struct" => {
                    // Look ahead for name
                    if i + 1 < tokens.len() {
                        if let CppTok::Identifier(name) = &tokens[i + 1] {
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
                CppTok::Keyword(kw) if kw == "enum" => {
                    spans.push(Span {
                        kind: SpanKind::ClassDecl,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: "enum".to_string(),
                    });
                }
                CppTok::Keyword(kw) if kw == "namespace" => {
                    spans.push(Span {
                        kind: SpanKind::ModuleDecl,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: "namespace".to_string(),
                    });
                }
                CppTok::Keyword(kw) if kw == "template" => {
                    spans.push(Span {
                        kind: SpanKind::GenericParam,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: "template".to_string(),
                    });
                }
                CppTok::Keyword(kw) if kw == "if" || kw == "else" => {
                    spans.push(Span {
                        kind: SpanKind::IfStmt,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: kw.to_string(),
                    });
                }
                CppTok::Keyword(kw) if kw == "for" || kw == "while" => {
                    spans.push(Span {
                        kind: SpanKind::ForLoop,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: kw.to_string(),
                    });
                }
                CppTok::Keyword(kw) if kw == "switch" => {
                    spans.push(Span {
                        kind: SpanKind::Block,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: "switch".to_string(),
                    });
                }
                CppTok::Keyword(kw) if kw == "case" => {
                    spans.push(Span {
                        kind: SpanKind::Pattern,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: "case".to_string(),
                    });
                }
                CppTok::Keyword(kw) if kw == "try" => {
                    spans.push(Span {
                        kind: SpanKind::TryBlock,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: "try".to_string(),
                    });
                }
                CppTok::Keyword(kw) if kw == "catch" => {
                    spans.push(Span {
                        kind: SpanKind::CatchBlock,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: "catch".to_string(),
                    });
                }
                CppTok::Preprocessor(dir) if dir.starts_with("#include") => {
                    spans.push(Span {
                        kind: SpanKind::ImportStatement,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: dir.clone(),
                    });
                }
                CppTok::Preprocessor(dir) if dir.starts_with("#define") => {
                    spans.push(Span {
                        kind: SpanKind::ConstStmt,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: dir.clone(),
                    });
                }
                CppTok::StringLiteral(content) => {
                    spans.push(Span {
                        kind: SpanKind::LitStr,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: format!("\"{}\"", content),
                    });
                }
                CppTok::CharLiteral(ch) => {
                    spans.push(Span {
                        kind: SpanKind::LitChar,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: format!("'{}'", ch),
                    });
                }
                CppTok::NumberLiteral(content) => {
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
            if let CppTok::Identifier(name) = &tokens[i] {
                // Check if next token is '(' — potential function definition
                if i + 1 < tokens.len() {
                    if let CppTok::Punctuation(p) = &tokens[i + 1] {
                        if p == "(" {
                            // Look back for return type (identifier or keyword)
                            let has_return_type = i > 0 && matches!(
                                &tokens[i - 1],
                                CppTok::Keyword(_) | CppTok::Identifier(_)
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
        semantic_tags.extend(self.detect_template_patterns(source));
        semantic_tags.extend(self.detect_smart_pointer_patterns(source));
        semantic_tags.extend(self.detect_modern_cpp_patterns(source));

        SpanTree {
            spans,
            source: source.to_string(),
            language: "cpp".to_string(),
            semantic_tags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpp_adapter_name() {
        let adapter = CppAdapter::new();
        assert_eq!(adapter.name(), "cpp");
    }

    #[test]
    fn test_tokenize_class_declaration() {
        let adapter = CppAdapter::new();
        let tree = adapter.tokenize_and_adapt("class MyClass { public: void method(); };");

        let has_class_decl = tree.spans.iter().any(|s| s.kind == SpanKind::ClassDecl);
        assert!(has_class_decl);
    }

    #[test]
    fn test_tokenize_namespace() {
        let adapter = CppAdapter::new();
        let tree = adapter.tokenize_and_adapt("namespace myns { int x; }");

        let has_module = tree.spans.iter().any(|s| s.kind == SpanKind::ModuleDecl);
        assert!(has_module);
    }

    #[test]
    fn test_tokenize_template() {
        let adapter = CppAdapter::new();
        let tree = adapter.tokenize_and_adapt("template <typename T> class Container {}");

        let has_generic = tree.spans.iter().any(|s| s.kind == SpanKind::GenericParam);
        assert!(has_generic);
    }

    #[test]
    fn test_tokenize_try_catch() {
        let adapter = CppAdapter::new();
        let tree = adapter.tokenize_and_adapt("try { } catch (const std::exception& e) { }");

        let has_try = tree.spans.iter().any(|s| s.kind == SpanKind::TryBlock);
        let has_catch = tree.spans.iter().any(|s| s.kind == SpanKind::CatchBlock);
        assert!(has_try && has_catch);
    }

    #[test]
    fn test_tokenize_include_directive() {
        let adapter = CppAdapter::new();
        let tree = adapter.tokenize_and_adapt("#include <vector>");

        let has_import = tree.spans.iter().any(|s| s.kind == SpanKind::ImportStatement);
        assert!(has_import);
    }

    #[test]
    fn test_cross_language_compatibility() {
        let cpp_adapter = CppAdapter::new();
        let c_adapter = crate::adapters::CAdapter::new();

        // Both should detect functions
        let cpp_tree = cpp_adapter.tokenize_and_adapt("void hello() {}");
        let c_tree = c_adapter.tokenize_and_adapt("void hello() {}");

        assert!(cpp_tree.spans.iter().any(|s| s.kind == SpanKind::FnDecl));
        assert!(c_tree.spans.iter().any(|s| s.kind == SpanKind::FnDecl));
    }
}