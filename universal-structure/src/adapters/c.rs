//! C adapter for universal structural token representation.
//!
//! Uses regex-based pattern matching to identify key C constructs and map them
//! to universal `SpanKind` variants. This follows the same approach as the JavaScript
//! and Python adapters, demonstrating the pluggable adapter architecture without
//! requiring a full C parser.

use crate::{LanguageAdapter, Span, SpanKind, SpanTree};
use std::collections::HashMap;

pub(crate) mod patterns;
pub(crate) mod tokenizer;

pub use tokenizer::CTok;

/// C adapter implementing the `LanguageAdapter` trait.
#[derive(Default)]
pub struct CAdapter;

impl CAdapter {
    #[must_use]
    pub fn new() -> Self {
        CAdapter
    }
}

impl LanguageAdapter for CAdapter {
    fn name(&self) -> &'static str {
        "c"
    }

    fn tokenize_and_adapt(&self, source: &str) -> SpanTree {
        let tokens = tokenizer::tokenize(source);

        // Build simplified span tree from tokens
        let mut spans = Vec::new();

        for (i, token) in tokens.iter().enumerate() {
            match token {
                CTok::Keyword(kw) if kw == "struct" || kw == "enum" || kw == "union" => {
                    // Look ahead for name
                    if i + 1 < tokens.len() && matches!(&tokens[i + 1], CTok::Identifier(_)) {
                        let name = match &tokens[i + 1] {
                            CTok::Identifier(n) => n.clone(),
                            _ => unreachable!(),
                        };
                        spans.push(Span {
                            kind: SpanKind::ClassDecl,
                            open_token: None,
                            body: Vec::new(),
                            close_token: None,
                            source: format!("{kw} {name}"),
                        });
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
                        source: kw.clone(),
                    });
                }
                CTok::Keyword(kw) if kw == "for" || kw == "while" => {
                    spans.push(Span {
                        kind: SpanKind::ForLoop,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: kw.clone(),
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
                        source: format!("\"{content}\""),
                    });
                }
                CTok::CharLiteral(ch) => {
                    spans.push(Span {
                        kind: SpanKind::LitChar,
                        open_token: None,
                        body: Vec::new(),
                        close_token: None,
                        source: format!("'{ch}'"),
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
                if i + 1 < tokens.len() && matches!(&tokens[i + 1], CTok::Punctuation(_)) {
                    let p = match &tokens[i + 1] {
                        CTok::Punctuation(p) => p.clone(),
                        _ => unreachable!(),
                    };
                    if p == "(" {
                        // Look back for return type (identifier or keyword)
                        let has_return_type = i > 0
                            && matches!(&tokens[i - 1], CTok::Keyword(_) | CTok::Identifier(_));

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
            i += 1;
        }

        // Add semantic annotations
        let mut semantic_tags = HashMap::new();
        semantic_tags.extend(patterns::detect_pointer_patterns(source));
        semantic_tags.extend(patterns::detect_memory_patterns(source));
        semantic_tags.extend(patterns::detect_preprocessor_patterns(source));

        SpanTree {
            spans,
            source: source.to_string(),
            language: "c".to_string(),
            semantic_tags,
        }
    }
}

#[cfg(test)]
mod tests;
