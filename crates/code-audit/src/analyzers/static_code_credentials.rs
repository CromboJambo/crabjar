//! Hardcoded credential detection for StaticCodeAnalyzer.

use crate::types::*;
use super::static_code::find_line_for_var;
use syn::{Expr, Item, parse_file};

impl super::static_code::StaticCodeAnalyzer {
    /// AST-based hardcoded credential detection.
    /// Uses syn to parse Rust into AST and find literal string values assigned to
    /// variables with secret-like names. Returns Finding objects directly with proper location info.
    pub(crate) fn find_hardcoded_credentials_ast(&self, path: &str, content: &str) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Parse the Rust source into an AST
        let file = match parse_file(content) {
            Ok(f) => f,
            Err(_) => return findings,
        };

        // Walk all items in the file
        for item in &file.items {
            match item {
                Item::Const(item_const) => {
                    if let Expr::Lit(expr_lit) = &*item_const.expr
                        && let syn::Lit::Str(lit_str) = &expr_lit.lit
                    {
                        let name = item_const.ident.to_string();
                        if self.is_secret_name(&name)
                            && self.looks_like_secret_value(&lit_str.value())
                        {
                            // Find line by searching for the variable declaration
                            let line_num = find_line_for_var(content, &name);
                            findings.push(self.create_credential_finding(
                                path,
                                &name,
                                lit_str.value(),
                                line_num,
                            ));
                        }
                    }
                }
                Item::Static(item_static) => {
                    if let Expr::Lit(expr_lit) = &*item_static.expr
                        && let syn::Lit::Str(lit_str) = &expr_lit.lit
                    {
                        let name = item_static.ident.to_string();
                        if self.is_secret_name(&name)
                            && self.looks_like_secret_value(&lit_str.value())
                        {
                            let line_num = find_line_for_var(content, &name);
                            findings.push(self.create_credential_finding(
                                path,
                                &name,
                                lit_str.value(),
                                line_num,
                            ));
                        }
                    }
                }
                Item::Fn(item_fn) => {
                    // Check for hardcoded credentials in function bodies (assignments)
                    self.check_function_body_ast(path, content, &item_fn.block, &mut findings);
                }
                _ => {}
            }
        }

        findings
    }

    /// Check function body for hardcoded credential assignments.
    fn check_function_body_ast(
        &self,
        path: &str,
        content: &str,
        block: &syn::Block,
        findings: &mut Vec<Finding>,
    ) {
        for stmt in &block.stmts {
            if let syn::Stmt::Local(local) = stmt {
                // Check if this is a variable assignment with a string literal
                if let Some(init) = &local.init
                    && let Expr::Lit(expr_lit) = &*init.expr
                    && let syn::Lit::Str(lit_str) = &expr_lit.lit
                {
                    // Check each identifier in the pattern
                    if let syn::Pat::Ident(pat_ident) = &local.pat {
                        let name = pat_ident.ident.to_string();
                        if self.is_secret_name(&name)
                            && self.looks_like_secret_value(&lit_str.value())
                        {
                            let line_num = find_line_for_var(content, &name);
                            findings.push(self.create_credential_finding(
                                path,
                                &name,
                                lit_str.value(),
                                line_num,
                            ));
                        }
                    }
                }
            }
        }
    }

    fn create_credential_finding(
        &self,
        path: &str,
        name: &str,
        value: String,
        line_num: usize,
    ) -> Finding {
        Finding {
            id: format!("hardcoded-credential-{}-{}", path, name),
            title: format!("Hardcoded credential in {}", path),
            description: format!("Variable '{}' contains a hardcoded secret value", name),
            severity: Severity::High,
            cwe: Some("CWE-798".to_string()),
            analyzer: AnalyzerType::StaticCode,
            evidence: vec![Evidence {
                kind: "code_snippet".to_string(),
                data: serde_json::json!({
                    "file": path,
                    "line": line_num,
                    "variable": name,
                    "value_preview": if value.len() > 20 { &value[..20] } else { &value },
                }),
                location: Some(format!("{}:{}", path, line_num)),
            }],
            remediation: format!("Store '{}' in environment variable or secure vault", name),
            inference_validated: false,
            confidence: None,
        }
    }

    /// Check if an identifier name suggests it holds a secret.
    fn is_secret_name(&self, name: &str) -> bool {
        let lower = name.to_lowercase();
        [
            "password",
            "passwd",
            "pass",
            "secret",
            "api_key",
            "apikey",
            "api_secret",
            "token",
            "access_token",
            "auth_token",
            "private_key",
            "secret_key",
            "encryption_key",
            "credential",
            "credentials",
        ]
        .iter()
        .any(|keyword| lower.contains(keyword))
    }

    /// Check if a string value looks like an actual secret (not just an error message or placeholder).
    pub(crate) fn looks_like_secret_value(&self, value: &str) -> bool {
        // Skip empty strings and common placeholders
        if value.is_empty() || value == "<secret>" || value == "REDACTED" {
            return false;
        }

        // Skip error messages (usually longer and contain spaces/punctuation)
        if value.contains("failed") || value.contains("error") || value.contains("unable to") {
            return false;
        }

        // Look for patterns that suggest actual secrets
        // API keys often start with specific prefixes
        if value.starts_with("sk-") || value.starts_with("ghp_") || value.starts_with("Bearer ") {
            return true;
        }

        // Short strings assigned to secret-named variables are suspicious
        if value.len() < 100 && !value.contains('\n') {
            return true;
        }

        false
    }
}
