//! Binary analysis adapter for compiled ELF binaries.
//!
//! Parses ELF files to extract function symbols, imports, exports, and call
//! graph information, then maps the structure to universal structural tokens.
//! This enables binary vs source diffing and architectural analysis of
//! compiled artifacts without requiring source code.

use crate::{EvaluationTarget, LanguageAdapter, Span, SpanKind, SpanTree};
use object::{Object, ObjectSection, ObjectSymbol, SymbolKind};
use std::collections::HashMap;

/// Binary analysis adapter that parses ELF files and extracts structural
/// information for JEV evaluation.
pub struct BinaryAdapter {
    /// Path to the binary file being analyzed.
    pub path: String,
}

impl BinaryAdapter {
    /// Create a new binary adapter for the given ELF file path.
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }

    /// Parse an ELF binary and extract its structural information.
    pub fn parse_elf(&self) -> Result<BinaryStructure, String> {
        let contents = std::fs::read(&self.path).map_err(|e| {
            format!("Failed to read binary {}: {}", self.path, e)
        })?;

        let obj = object::File::parse(&contents[..]).map_err(|e| {
            format!("Failed to parse ELF file {}: {}", self.path, e)
        })?;

        let mut structure = BinaryStructure {
            path: self.path.clone(),
            functions: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            symbols: Vec::new(),
            sections: Vec::new(),
        };

        // Extract functions from symbol table
        for sym in obj.symbols() {
            if let Ok(name) = sym.name() {
                if !name.is_empty() {
                    structure.symbols.push(SymbolInfo {
                        name: name.to_string(),
                        address: sym.address() as usize,
                        size: sym.size() as usize,
                        is_function: sym.kind() == SymbolKind::Text,
                        is_global: sym.is_global(),
                        section: sym.section_index().map(|s| s.0),
                    });

                    // Classify by visibility and type
                    if sym.kind() == SymbolKind::Text && sym.is_global() {
                        structure.exports.push(name.to_string());
                    } else if sym.kind() == SymbolKind::Text {
                        structure.functions.push(FunctionInfo {
                            name: name.to_string(),
                            address: sym.address() as usize,
                            size: sym.size() as usize,
                        });
                    }
                }
            }
        }

        // Note: Dynamic symbol (import) extraction requires format-specific parsing.
        // For now, we rely on regular symbol table analysis which works across formats.

        // Extract section information
        for section in obj.sections() {
            if let Ok(name) = section.name() {
                structure.sections.push(SectionInfo {
                    name: name.to_string(),
                    size: section.size() as usize,
                    address: section.address() as usize,
                });
            }
        }

        Ok(structure)
    }

    /// Convert binary structure to universal span tree for JEV evaluation.
    pub fn to_span_tree(&self, structure: &BinaryStructure) -> SpanTree {
        let mut spans = Vec::new();

        // Module-level span representing the entire binary
        let mut module = Span::new(SpanKind::ModuleDecl);
        module.source = format!("binary: {}", self.path);

        // Add function declarations as FnDecl spans
        for func in &structure.functions {
            let mut fn_span = Span::new(SpanKind::FnDecl);
            fn_span.source = format!("fn {} (size={})", func.name, func.size);
            fn_span.open_token = Some(
                crate::StructuralToken::new(SpanKind::FnDecl, &func.name, [
                    func.address,
                    func.address + func.size,
                ])
                .with_tag("binary_address", format!("{:#x}", func.address))
                .with_tag("binary_size", func.size.to_string()),
            );

            module.body.push(fn_span);
        }

        // Add exports as ExportStatement spans
        for export in &structure.exports {
            let mut export_span = Span::new(SpanKind::ExportStatement);
            export_span.source = format!("export {}", export);
            export_span.open_token = Some(crate::StructuralToken::new(
                SpanKind::ExportStatement,
                export,
                [0, 0],
            ));

            module.body.push(export_span);
        }

        // Add imports as ImportStatement spans
        for import in &structure.imports {
            let mut import_span = Span::new(SpanKind::ImportStatement);
            import_span.source = format!("import {}", import);
            import_span.open_token = Some(crate::StructuralToken::new(
                SpanKind::ImportStatement,
                import,
                [0, 0],
            ));

            module.body.push(import_span);
        }

        spans.push(module);

        SpanTree {
            spans,
            source: self.path.clone(),
            language: "binary".to_string(),
            semantic_tags: HashMap::from([
                ("format".to_string(), "elf".to_string()),
                (
                    "function_count".to_string(),
                    structure.functions.len().to_string(),
                ),
                (
                    "import_count".to_string(),
                    structure.imports.len().to_string(),
                ),
                (
                    "export_count".to_string(),
                    structure.exports.len().to_string(),
                ),
            ]),
        }
    }

    /// Convert binary structure to evaluation targets for JEV.
    pub fn to_evaluation_targets(&self, structure: &BinaryStructure) -> Vec<EvaluationTarget> {
        let mut targets = Vec::new();

        for func in &structure.functions {
            targets.push(EvaluationTarget {
                structure: SpanKind::FnDecl,
                range: [func.address, func.address + func.size],
                source: format!("fn {} (binary, size={})", func.name, func.size),
                name: Some(func.name.clone()),
                language: Some("binary".to_string()),
                semantic_tags: HashMap::from([
                    ("binary_address".to_string(), format!("{:#x}", func.address)),
                    ("binary_size".to_string(), func.size.to_string()),
                ]),
                error_handling: None,
                mutability: None,
            });
        }

        targets
    }

    /// Compare two binary structures and identify differences.
    pub fn compare_binaries(&self, a: &BinaryStructure, b: &BinaryStructure) -> BinaryDiff {
        let mut diff = BinaryDiff {
            added_functions: Vec::new(),
            removed_functions: Vec::new(),
            modified_functions: Vec::new(),
            added_imports: Vec::new(),
            removed_imports: Vec::new(),
        };

        // Find added/removed functions
        let a_func_names: std::collections::HashSet<&str> =
            a.functions.iter().map(|f| f.name.as_str()).collect();
        let b_func_names: std::collections::HashSet<&str> =
            b.functions.iter().map(|f| f.name.as_str()).collect();

        for name in &b_func_names {
            if !a_func_names.contains(name) {
                diff.added_functions.push(name.to_string());
            }
        }

        for name in &a_func_names {
            if !b_func_names.contains(name) {
                diff.removed_functions.push(name.to_string());
            }
        }

        // Find modified functions (size changes indicate recompilation)
        let a_funcs: std::collections::HashMap<&str, usize> =
            a.functions.iter().map(|f| (&f.name[..], f.size)).collect();
        for b_func in &b.functions {
            if let Some(&a_size) = a_funcs.get(b_func.name.as_str()) {
                if a_size != b_func.size {
                    diff.modified_functions
                        .push((b_func.name.clone(), a_size, b_func.size));
                }
            }
        }

        // Find added/removed imports
        let a_import_names: std::collections::HashSet<&str> =
            a.imports.iter().map(|i| i.as_str()).collect();
        let b_import_names: std::collections::HashSet<&str> =
            b.imports.iter().map(|i| i.as_str()).collect();

        for name in &b_import_names {
            if !a_import_names.contains(name) {
                diff.added_imports.push(name.to_string());
            }
        }

        for name in &a_import_names {
            if !b_import_names.contains(name) {
                diff.removed_imports.push(name.to_string());
            }
        }

        diff
    }
}

impl LanguageAdapter for BinaryAdapter {
    fn name(&self) -> &str {
        "binary"
    }

    fn tokenize_and_adapt(&self, _source: &str) -> SpanTree {
        // For binary adapter, "source" is the path to the ELF file
        match self.parse_elf() {
            Ok(structure) => self.to_span_tree(&structure),
            Err(e) => SpanTree {
                spans: Vec::new(),
                source: e,
                language: "binary".to_string(),
                semantic_tags: HashMap::from([("error".to_string(), "parse_failed".to_string())]),
            },
        }
    }
}

/// Extracted binary structure information.
#[derive(Debug)]
pub struct BinaryStructure {
    pub path: String,
    pub functions: Vec<FunctionInfo>,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub symbols: Vec<SymbolInfo>,
    pub sections: Vec<SectionInfo>,
}

/// Information about a function in the binary.
#[derive(Debug)]
pub struct FunctionInfo {
    pub name: String,
    pub address: usize,
    pub size: usize,
}

/// Symbol information extracted from the binary.
#[derive(Debug)]
pub struct SymbolInfo {
    pub name: String,
    pub address: usize,
    pub size: usize,
    pub is_function: bool,
    pub is_global: bool,
    pub section: Option<usize>,
}

/// Section information from the binary.
#[derive(Debug)]
pub struct SectionInfo {
    pub name: String,
    pub size: usize,
    pub address: usize,
}

/// Differences between two binary structures.
#[derive(Debug)]
pub struct BinaryDiff {
    pub added_functions: Vec<String>,
    pub removed_functions: Vec<String>,
    pub modified_functions: Vec<(String, usize, usize)>, // (name, old_size, new_size)
    pub added_imports: Vec<String>,
    pub removed_imports: Vec<String>,
}

impl BinaryDiff {
    /// Check if the diff is empty (binaries are structurally identical).
    pub fn is_empty(&self) -> bool {
        self.added_functions.is_empty()
            && self.removed_functions.is_empty()
            && self.modified_functions.is_empty()
            && self.added_imports.is_empty()
            && self.removed_imports.is_empty()
    }

    /// Generate a human-readable summary of the diff.
    pub fn summary(&self) -> String {
        let mut lines = Vec::new();

        if !self.added_functions.is_empty() {
            lines.push(format!(
                "Added functions: {}",
                self.added_functions.join(", ")
            ));
        }
        if !self.removed_functions.is_empty() {
            lines.push(format!(
                "Removed functions: {}",
                self.removed_functions.join(", ")
            ));
        }
        if !self.modified_functions.is_empty() {
            let modified: Vec<String> = self
                .modified_functions
                .iter()
                .map(|(name, old_size, new_size)| {
                    format!("{} (size {} -> {})", name, old_size, new_size)
                })
                .collect();
            lines.push(format!("Modified functions: {}", modified.join(", ")));
        }
        if !self.added_imports.is_empty() {
            lines.push(format!(
                "Added imports: {}",
                self.added_imports.join(", ")
            ));
        }
        if !self.removed_imports.is_empty() {
            lines.push(format!(
                "Removed imports: {}",
                self.removed_imports.join(", ")
            ));
        }

        if lines.is_empty() {
            "Binaries are structurally identical".to_string()
        } else {
            lines.join("\n")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_adapter_creation() {
        let adapter = BinaryAdapter::new("test_binary");
        assert_eq!(adapter.name(), "binary");
        assert_eq!(adapter.path, "test_binary");
    }

    #[test]
    fn test_binary_diff_empty() {
        let diff = BinaryDiff {
            added_functions: Vec::new(),
            removed_functions: Vec::new(),
            modified_functions: Vec::new(),
            added_imports: Vec::new(),
            removed_imports: Vec::new(),
        };

        assert!(diff.is_empty());
        assert_eq!(diff.summary(), "Binaries are structurally identical");
    }

    #[test]
    fn test_binary_diff_with_changes() {
        let diff = BinaryDiff {
            added_functions: vec!["new_func".to_string()],
            removed_functions: vec!["old_func".to_string()],
            modified_functions: vec![("changed".to_string(), 100, 200)],
            added_imports: vec!["new_import".to_string()],
            removed_imports: Vec::new(),
        };

        assert!(!diff.is_empty());
        let summary = diff.summary();
        assert!(summary.contains("Added functions"));
        assert!(summary.contains("Removed functions"));
        assert!(summary.contains("Modified functions"));
        assert!(summary.contains("changed (size 100 -> 200)"));
    }

    #[test]
    fn test_to_span_tree_structure() {
        let structure = BinaryStructure {
            path: "test".to_string(),
            functions: vec![FunctionInfo {
                name: "main".to_string(),
                address: 0x1000,
                size: 100,
            }],
            imports: vec!["puts".to_string()],
            exports: vec!["main".to_string()],
            symbols: Vec::new(),
            sections: Vec::new(),
        };

        let adapter = BinaryAdapter::new("test");
        let tree = adapter.to_span_tree(&structure);

        assert_eq!(tree.language, "binary");
        assert_eq!(tree.spans.len(), 1); // Module span
        assert_eq!(tree.spans[0].kind, SpanKind::ModuleDecl);
        assert_eq!(tree.spans[0].body.len(), 3); // fn + export + import

        // Verify semantic tags
        assert_eq!(
            tree.semantic_tags.get("format").unwrap(),
            &"elf".to_string()
        );
        assert_eq!(
            tree.semantic_tags.get("function_count").unwrap(),
            &"1".to_string()
        );
    }

    #[test]
    fn test_to_evaluation_targets() {
        let structure = BinaryStructure {
            path: "test".to_string(),
            functions: vec![FunctionInfo {
                name: "process_data".to_string(),
                address: 0x2000,
                size: 500,
            }],
            imports: Vec::new(),
            exports: Vec::new(),
            symbols: Vec::new(),
            sections: Vec::new(),
        };

        let adapter = BinaryAdapter::new("test");
        let targets = adapter.to_evaluation_targets(&structure);

        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].name.as_ref().unwrap(), "process_data");
        assert_eq!(targets[0].language.as_ref().unwrap(), "binary");
        assert_eq!(targets[0].structure, SpanKind::FnDecl);
    }
}