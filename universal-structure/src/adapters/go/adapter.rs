//! Go LanguageAdapter implementation: converts source to universal SpanTree.

use crate::{LanguageAdapter, Span, SpanKind, SpanTree};
use std::collections::HashMap;

use super::GoTok;
use super::patterns::{detect_channel_patterns, detect_error_patterns, detect_goroutine_patterns};
use super::tokenizer::tokenize;

/// Go adapter implementing the `LanguageAdapter` trait.
pub struct GoAdapter;

impl Default for GoAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl GoAdapter {
    #[must_use]
    pub fn new() -> Self {
        GoAdapter
    }
}

impl LanguageAdapter for GoAdapter {
    fn name(&self) -> &'static str {
        "go"
    }

    fn tokenize_and_adapt(&self, source: &str) -> SpanTree {
        let tokens = tokenize(source);

        // Build simplified span tree from tokens
        let mut spans = Vec::new();

        for (i, token) in tokens.iter().enumerate() {
            match token {
                GoTok::Keyword(kw) if kw == "package" => {
                    // Look ahead for package name
                    if i + 1 < tokens.len()
                        && let GoTok::Identifier(name) = &tokens[i + 1]
                    {
                        spans.push(Span {
                            kind: SpanKind::ModuleDecl,
                            open_token: None,
                            body: Vec::new(),
                            close_token: None,
                            source: format!("package {name}"),
                        });
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
                    if i + 1 < tokens.len()
                        && let GoTok::Identifier(name) = &tokens[i + 1]
                    {
                        spans.push(Span {
                            kind: SpanKind::FnDecl,
                            open_token: None,
                            body: Vec::new(),
                            close_token: None,
                            source: format!("func {name}"),
                        });
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
                        source: kw.clone(),
                    });
                }
                GoTok::Keyword(kw) if kw == "for" || kw == "range" => {
                    spans.push(Span {
                        kind: SpanKind::ForLoop,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: kw.clone(),
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
                        source: format!("\"{content}\""),
                    });
                }
                GoTok::CharLiteral(ch) => {
                    spans.push(Span {
                        kind: SpanKind::LitChar,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: format!("'{ch}'"),
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
        semantic_tags.extend(detect_goroutine_patterns(source));
        semantic_tags.extend(detect_channel_patterns(source));
        semantic_tags.extend(detect_error_patterns(source));

        SpanTree {
            spans,
            source: source.to_string(),
            language: "go".to_string(),
            semantic_tags,
        }
    }
}
