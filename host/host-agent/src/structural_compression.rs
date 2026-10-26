//! Structural code compression using pesti-structural-tokenizer.
//!
//! Compresses code-heavy observations by eliding large function bodies while
//! preserving declarations and control flow structure. Uses the budget-aware
//! tokenization mode of pesti-structural-tokenizer.

use pesti_structural_tokenizer::{Budget, StructuralTokenizer};

/// Compress Rust source code using structural tokenization with budget constraints.
///
/// Large function bodies are elided into markers while preserving declarations,
/// control flow structure, and semantic annotations. Returns the compressed
/// representation along with information about what was elided.
pub fn compress_code(src: &str, max_tokens: usize) -> CompressedCode {
    let tokenizer = StructuralTokenizer::new();
    let budget = Budget {
        max_tokens,
        min_body_stmts: 3, // Elide bodies with >3 statements
    };

    match tokenizer.tokenize_with_budget(src, budget) {
        Ok(emissions) => {
            let mut compressed = String::new();
            let mut elided_count = 0;
            let mut total_elided_nodes = 0;

            for emission in &emissions {
                match emission {
                    pesti_structural_tokenizer::Emission::Node(token) => {
                        compressed.push_str(&format!("[{}] ", token.kind));
                        if !token.text.is_empty() {
                            // Truncate long text values
                            let display = if token.text.len() > 40 {
                                format!("{}...", &token.text[..40])
                            } else {
                                token.text.clone()
                            };
                            compressed.push_str(&display);
                        }
                        compressed.push(' ');
                    }
                    pesti_structural_tokenizer::Emission::Elided(span) => {
                        elided_count += 1;
                        total_elided_nodes += span.node_count;
                        let summary_parts: Vec<String> = span
                            .kind_summary
                            .iter()
                            .take(3) // Limit to top 3 kinds
                            .map(|(k, n)| format!("{}x{}", k, n))
                            .collect();
                        compressed.push_str(&format!(
                            "[ELIDED {} nodes: {}]",
                            span.node_count,
                            summary_parts.join(", ")
                        ));
                        compressed.push(' ');
                    }
                }
            }

            CompressedCode {
                compressed,
                elided_count,
                total_elided_nodes,
                original_tokens: emissions.len(),
            }
        }
        Err(e) => {
            // Fall back to simple truncation on parse error
            let truncated = if src.len() > 200 {
                format!("{}...", &src[..200])
            } else {
                src.to_string()
            };
            CompressedCode {
                compressed: format!("[PARSE ERROR: {}] {}", e, truncated),
                elided_count: 0,
                total_elided_nodes: 0,
                original_tokens: 0,
            }
        }
    }
}

/// Result of structural code compression.
pub struct CompressedCode {
    /// The compressed token sequence as a string.
    pub compressed: String,
    /// Number of elision markers inserted.
    pub elided_count: usize,
    /// Total number of nodes collapsed into elision markers.
    pub total_elided_nodes: usize,
    /// Number of structural tokens in the original code.
    pub original_tokens: usize,
}

impl CompressedCode {
    /// Returns true if any elision occurred (compression happened).
    pub fn was_compressed(&self) -> bool {
        self.elided_count > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_large_function() {
        let src = "fn big_function() {
            let x = 1;
            let y = 2;
            let z = 3;
            let a = 4;
            let b = 5;
            let c = 6;
            return x + y + z + a + b + c;
        }";

        let result = compress_code(src, 5);
        assert!(result.was_compressed());
        // Should have function declaration + elision marker
        assert!(result.compressed.contains("FN_DECL"));
    }

    #[test]
    fn test_compress_small_function() {
        let src = "fn small() { return 42; }";
        let result = compress_code(src, 100);
        // Small body should not be elided
        assert!(!result.was_compressed());
    }

    #[test]
    fn test_compress_preserves_structure() {
        let src = "struct Point { x: i32, y: i32 }
fn add(a: i32, b: i32) -> i32 { return a + b; }";

        let result = compress_code(src, 100);
        assert!(result.compressed.contains("STRUCT_DECL"));
        assert!(result.compressed.contains("FN_DECL"));
    }
}
