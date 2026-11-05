//! C++ LanguageAdapter implementation: converts source to universal SpanTree.

use crate::{LanguageAdapter, Span, SpanKind, SpanTree};
use std::collections::HashMap;

use super::CppTok;
use super::patterns::{
    detect_modern_cpp_patterns, detect_smart_pointer_patterns, detect_template_patterns,
};
use super::tokenizer::tokenize;

/// C++ adapter implementing the `LanguageAdapter` trait.
pub struct CppAdapter;

impl Default for CppAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl CppAdapter {
    #[must_use]
    pub fn new() -> Self {
        CppAdapter
    }
}

impl LanguageAdapter for CppAdapter {
    fn name(&self) -> &'static str {
        "cpp"
    }

    fn tokenize_and_adapt(&self, source: &str) -> SpanTree {
        let tokens = tokenize(source);

        // Build simplified span tree from tokens
        let mut spans = Vec::new();

        for (i, token) in tokens.iter().enumerate() {
            match token {
                CppTok::Keyword(kw) if kw == "class" || kw == "struct" => {
                    // Look ahead for name
                    if i + 1 < tokens.len()
                        && let CppTok::Identifier(name) = &tokens[i + 1]
                    {
                        spans.push(Span {
                            kind: SpanKind::ClassDecl,
                            open_token: None,
                            body: Vec::new(),
                            close_token: None,
                            source: format!("{kw} {name}"),
                        });
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
                        source: kw.clone(),
                    });
                }
                CppTok::Keyword(kw) if kw == "for" || kw == "while" => {
                    spans.push(Span {
                        kind: SpanKind::ForLoop,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: kw.clone(),
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
                        source: format!("\"{content}\""),
                    });
                }
                CppTok::CharLiteral(ch) => {
                    spans.push(Span {
                        kind: SpanKind::LitChar,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: format!("'{ch}'"),
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
                if i + 1 < tokens.len()
                    && let CppTok::Punctuation(p) = &tokens[i + 1]
                    && p == "("
                {
                    // Look back for return type (identifier or keyword)
                    let has_return_type = i > 0
                        && matches!(&tokens[i - 1], CppTok::Keyword(_) | CppTok::Identifier(_));

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
            i += 1;
        }

        // Add semantic annotations
        let mut semantic_tags = HashMap::new();
        semantic_tags.extend(detect_template_patterns(source));
        semantic_tags.extend(detect_smart_pointer_patterns(source));
        semantic_tags.extend(detect_modern_cpp_patterns(source));

        SpanTree {
            spans,
            source: source.to_string(),
            language: "cpp".to_string(),
            semantic_tags,
        }
    }
}
