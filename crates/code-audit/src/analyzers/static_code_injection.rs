//! Injection detection for StaticCodeAnalyzer — SQL, command, and path traversal.

use crate::types::*;
use super::static_code::find_line_for_var;
use syn::{Expr, Item, parse_file};

impl super::static_code::StaticCodeAnalyzer {
    /// Detect SQL injection patterns using AST analysis.
    /// Looks for string concatenation or format strings in query construction.
    pub(crate) fn find_sql_injection(&self, path: &str, content: &str) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Parse the Rust source into an AST
        let file = match parse_file(content) {
            Ok(f) => f,
            Err(_) => return findings,
        };

        // Walk all items in the file
        for item in &file.items {
            if let Item::Fn(item_fn) = item {
                self.check_function_for_sql_injection(
                    path,
                    content,
                    &item_fn.sig.ident.to_string(),
                    &item_fn.block,
                    &mut findings,
                );
            }
        }

        findings
    }

    /// Check a function body for SQL injection patterns.
    fn check_function_for_sql_injection(
        &self,
        path: &str,
        content: &str,
        func_name: &str,
        block: &syn::Block,
        findings: &mut Vec<Finding>,
    ) {
        let lines: Vec<&str> = content.lines().collect();

        for stmt in &block.stmts {
            if let syn::Stmt::Local(local) = stmt {
                // Check if this is a string concatenation that looks like SQL
                if let Some(init) = &local.init
                    && let Expr::Binary(bin_expr) = &*init.expr
                {
                    // Check if the operator is addition (string concatenation)
                    if matches!(bin_expr.op, syn::BinOp::Add(_)) {
                        // Check if either side contains SQL keywords
                        let left_sql = self.expr_contains_sql_keyword(&bin_expr.left);
                        let right_sql = self.expr_contains_sql_keyword(&bin_expr.right);

                        if left_sql || right_sql {
                            // Found potential SQL injection
                            let var_name = self.local_name(local);
                            let line_num = find_line_for_var(content, &var_name);

                            findings.push(Finding {
                                    id: format!("sql-injection-{}-{}-{}", path, func_name, line_num),
                                    title: "Potential SQL injection via string concatenation".to_string(),
                                    description: format!("SQL query in function '{}' constructed using string concatenation instead of parameterized query", func_name),
                                    severity: Severity::High,
                                    cwe: Some("CWE-89".to_string()),
                                    analyzer: AnalyzerType::StaticCode,
                                    evidence: vec![Evidence {
                                        kind: "code_snippet".to_string(),
                                        data: serde_json::json!({
                                            "file": path,
                                            "function": func_name,
                                            "line": line_num,
                                            "variable": self.local_name(local),
                                            "snippet": lines[line_num - 1].trim()
                                        }),
                                        location: Some(format!("{}:{}", path, line_num)),
                                    }],
                                    remediation: "Use parameterized queries with placeholders instead of string concatenation".to_string(),
                                    inference_validated: false,
                                    confidence: None,
                                });
                        }
                    }
                }
            }
        }
    }

    /// Check if an expression contains SQL keywords (recursively).
    fn expr_contains_sql_keyword(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Lit(expr_lit) => {
                if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                    let value = lit_str.value().to_uppercase();
                    return ["SELECT", "INSERT", "UPDATE", "DELETE", "FROM", "WHERE"]
                        .iter()
                        .any(|keyword| value.contains(keyword));
                }
            }
            Expr::Binary(bin_expr) => {
                // Check both sides of binary expression
                return self.expr_contains_sql_keyword(&bin_expr.left)
                    || self.expr_contains_sql_keyword(&bin_expr.right);
            }
            _ => {}
        }
        false
    }

    /// Detect command injection patterns using AST analysis.
    /// Looks for string concatenation in shell command construction.
    pub(crate) fn find_command_injection(&self, path: &str, content: &str) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Parse the Rust source into an AST
        let file = match parse_file(content) {
            Ok(f) => f,
            Err(_) => return findings,
        };

        // Walk all items in the file
        for item in &file.items {
            if let Item::Fn(item_fn) = item {
                self.check_function_for_command_injection(
                    path,
                    content,
                    &item_fn.sig.ident.to_string(),
                    &item_fn.block,
                    &mut findings,
                );
            }
        }

        findings
    }

    /// Check a function body for command injection patterns.
    fn check_function_for_command_injection(
        &self,
        path: &str,
        content: &str,
        func_name: &str,
        block: &syn::Block,
        findings: &mut Vec<Finding>,
    ) {
        let lines: Vec<&str> = content.lines().collect();

        for stmt in &block.stmts {
            if let syn::Stmt::Local(local) = stmt {
                // Check if this is a string concatenation that looks like a shell command
                if let Some(init) = &local.init
                    && let Expr::Binary(bin_expr) = &*init.expr
                {
                    // Check if the operator is addition (string concatenation)
                    if matches!(bin_expr.op, syn::BinOp::Add(_)) {
                        // Check if either side contains shell command indicators
                        let left_cmd = self.expr_contains_shell_command(&bin_expr.left);
                        let right_cmd = self.expr_contains_shell_command(&bin_expr.right);

                        if left_cmd || right_cmd {
                            // Found potential command injection
                            let var_name = self.local_name(local);
                            let line_num = find_line_for_var(content, &var_name);

                            findings.push(Finding {
                                    id: format!("cmd-injection-{}-{}-{}", path, func_name, line_num),
                                    title: "Potential command injection via string concatenation".to_string(),
                                    description: format!("Shell command in function '{}' constructed using string concatenation", func_name),
                                    severity: Severity::High,
                                    cwe: Some("CWE-78".to_string()),
                                    analyzer: AnalyzerType::StaticCode,
                                    evidence: vec![Evidence {
                                        kind: "code_snippet".to_string(),
                                        data: serde_json::json!({
                                            "file": path,
                                            "function": func_name,
                                            "line": line_num,
                                            "variable": var_name,
                                            "snippet": lines[line_num - 1].trim()
                                        }),
                                        location: Some(format!("{}:{}", path, line_num)),
                                    }],
                                    remediation: "Use execvp-style argument arrays instead of shell command strings".to_string(),
                                    inference_validated: false,
                                    confidence: None,
                                });
                        }
                    }
                }
            }
        }
    }

    /// Check if an expression contains shell command indicators (recursively).
    fn expr_contains_shell_command(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Lit(expr_lit) => {
                if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                    let value = lit_str.value();
                    // Look for common shell command patterns
                    return [
                        "ls ", "cat ", "grep ", "find ", "rm ", "cp ", "mv ", "chmod ", "chown ",
                        "mkdir ", "echo ", "cd ",
                    ]
                    .iter()
                    .any(|cmd| value.starts_with(cmd));
                }
            }
            Expr::Binary(bin_expr) => {
                // Check both sides of binary expression
                return self.expr_contains_shell_command(&bin_expr.left)
                    || self.expr_contains_shell_command(&bin_expr.right);
            }
            _ => {}
        }
        false
    }

    /// Detect path traversal patterns using AST analysis.
    /// Looks for string concatenation in file path construction.
    pub(crate) fn find_path_traversal(&self, path: &str, content: &str) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Parse the Rust source into an AST
        let file = match parse_file(content) {
            Ok(f) => f,
            Err(_) => return findings,
        };

        // Walk all items in the file
        for item in &file.items {
            if let Item::Fn(item_fn) = item {
                self.check_function_for_path_traversal(
                    path,
                    content,
                    &item_fn.sig.ident.to_string(),
                    &item_fn.block,
                    &mut findings,
                );
            }
        }

        findings
    }

    /// Check a function body for path traversal patterns.
    fn check_function_for_path_traversal(
        &self,
        path: &str,
        content: &str,
        func_name: &str,
        block: &syn::Block,
        findings: &mut Vec<Finding>,
    ) {
        let lines: Vec<&str> = content.lines().collect();

        for stmt in &block.stmts {
            if let syn::Stmt::Local(local) = stmt {
                // Check if this is a string concatenation that looks like a file path
                if let Some(init) = &local.init
                    && let Expr::Binary(bin_expr) = &*init.expr
                {
                    // Check if the operator is addition (string concatenation)
                    if matches!(bin_expr.op, syn::BinOp::Add(_)) {
                        // Check if either side contains path indicators
                        let left_path = self.expr_contains_path(&bin_expr.left);
                        let right_path = self.expr_contains_path(&bin_expr.right);

                        if left_path || right_path {
                            // Found potential path traversal
                            let var_name = self.local_name(local);
                            let line_num = find_line_for_var(content, &var_name);

                            findings.push(Finding {
                                    id: format!("path-traversal-{}-{}-{}", path, func_name, line_num),
                                    title: "Potential path traversal via string concatenation".to_string(),
                                    description: format!("File path in function '{}' constructed using string concatenation", func_name),
                                    severity: Severity::Medium,
                                    cwe: Some("CWE-22".to_string()),
                                    analyzer: AnalyzerType::StaticCode,
                                    evidence: vec![Evidence {
                                        kind: "code_snippet".to_string(),
                                        data: serde_json::json!({
                                            "file": path,
                                            "function": func_name,
                                            "line": line_num,
                                            "variable": var_name,
                                            "snippet": lines[line_num - 1].trim()
                                        }),
                                        location: Some(format!("{}:{}", path, line_num)),
                                    }],
                                    remediation: "Use Path::join() or validate and canonicalize user-provided paths".to_string(),
                                    inference_validated: false,
                                    confidence: None,
                                });
                        }
                    }
                }
            }
        }
    }

    /// Check if an expression contains file path indicators (recursively).
    fn expr_contains_path(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Lit(expr_lit) => {
                if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                    let value = lit_str.value();
                    // Look for path-like strings (containing / or ending with common extensions)
                    return value.contains('/')
                        || value.ends_with(".txt")
                        || value.ends_with(".log")
                        || value.ends_with(".json")
                        || value.ends_with(".csv");
                }
            }
            Expr::Binary(bin_expr) => {
                // Check both sides of binary expression
                return self.expr_contains_path(&bin_expr.left)
                    || self.expr_contains_path(&bin_expr.right);
            }
            _ => {}
        }
        false
    }
}
