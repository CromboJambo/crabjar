//! JavaScript source code analysis.
//!
//! Parses JS source to extract imports, exports, function declarations,
//! class definitions, global variable access, DOM API usage, network calls,
//! and obfuscation patterns. Uses pure Rust regex/string parsing — no external
//! JS parser dependency.

use analyze_common::{AnalysisProvenance, AnalysisResult, AnalyzeError, DoubtBlock};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;

// Version constant for provenance
const ANALYZE_JS_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Result of JavaScript analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsAnalysis {
    pub path: String,
    pub size_bytes: u64,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub functions: Vec<FunctionDecl>,
    pub classes: Vec<ClassDecl>,
    pub globals: Vec<String>,
    pub dom_apis: Vec<String>,
    pub network_calls: Vec<NetworkCall>,
    pub obfuscation: ObfuscationReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDecl {
    pub name: String,
    pub is_async: bool,
    pub is_exported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassDecl {
    pub name: String,
    pub extends: Option<String>,
    pub methods: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkCall {
    pub kind: String, // "fetch", "XMLHttpRequest", "WebSocket"
    pub url_pattern: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObfuscationReport {
    pub eval_usage: bool,
    pub base64_decoding: bool,
    pub string_concatenation: bool,
    pub suspicious_patterns: Vec<String>,
}

/// Analyze JavaScript source code.
pub fn analyze_js(path: &str) -> Result<AnalysisResult<JsAnalysis>, AnalyzeError> {
    let source = fs::read_to_string(path).map_err(|_| AnalyzeError::FileNotFound(path.to_string()))?;
    let size_bytes = source.len() as u64;

    // Extract imports
    let imports = extract_imports(&source);

    // Extract exports
    let exports = extract_exports(&source);

    // Extract function declarations
    let functions = extract_functions(&source, &exports);

    // Extract class definitions
    let classes = extract_classes(&source);

    // Detect globals
    let globals = detect_globals(&source);

    // Detect DOM APIs
    let dom_apis = detect_dom_apis(&source);

    // Detect network calls
    let network_calls = detect_network_calls(&source);

    // Detect obfuscation
    let obfuscation = detect_obfuscation(&source);

    let provenance = AnalysisProvenance::new("analyze-js", ANALYZE_JS_VERSION);
    let doubt = DoubtBlock::new("7d")
        .assume("file is valid JavaScript source")
        .blind_spot("minified code may have false positives for obfuscation")
        .blind_spot("dynamic imports evaluated at runtime not detected");

    let analysis = JsAnalysis {
        path: path.to_string(),
        size_bytes,
        imports,
        exports,
        functions,
        classes,
        globals,
        dom_apis,
        network_calls,
        obfuscation,
    };

    Ok(AnalysisResult::new(
        format!("js-{}", chrono::Utc::now().timestamp_millis()),
        provenance,
        analysis,
        doubt,
    ))
}

fn extract_imports(source: &str) -> Vec<String> {
    let mut imports = Vec::new();

    // ES6 import statements: import { x } from 'module', import x from 'module'
    let re_import_from = Regex::new(r#"import\s+(?:\{[^}]+\}|[a-zA-Z_$][\w$]*)\s+from\s+['"]([^'"]+)['"]"#).unwrap();
    for caps in re_import_from.captures_iter(source) {
        if let Some(module) = caps.get(1) {
            imports.push(module.as_str().to_string());
        }
    }
    // Side-effect imports: import './style.css' (no named/default binding)
    let re_side_effect = Regex::new(r#"import\s+['"]([^'"]+)['"]"#).unwrap();
    for caps in re_side_effect.captures_iter(source) {
        if let Some(module) = caps.get(1) {
            imports.push(module.as_str().to_string());
        }
    }

    // CommonJS require() calls
    let re_require = Regex::new(r#"require\s*\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap();
    for caps in re_require.captures_iter(source) {
        if let Some(module) = caps.get(1) {
            imports.push(module.as_str().to_string());
        }
    }

    imports.sort();
    imports.dedup();
    imports
}

fn extract_exports(source: &str) -> Vec<String> {
    let mut exports = HashSet::new();

    // export function name() {}
    let re_export_fn = Regex::new(r"export\s+(?:async\s+)?function\s+(\w+)").unwrap();
    for caps in re_export_fn.captures_iter(source) {
        if let Some(name) = caps.get(1) {
            exports.insert(name.as_str().to_string());
        }
    }

    // export class name {}
    let re_export_class = Regex::new(r"export\s+class\s+(\w+)").unwrap();
    for caps in re_export_class.captures_iter(source) {
        if let Some(name) = caps.get(1) {
            exports.insert(name.as_str().to_string());
        }
    }

    // export const/let/var name = ...
    let re_export_const = Regex::new(r"export\s+(?:const|let|var)\s+(\w+)").unwrap();
    for caps in re_export_const.captures_iter(source) {
        if let Some(name) = caps.get(1) {
            exports.insert(name.as_str().to_string());
        }
    }

    // module.exports = { name: ... } or shorthand { add, subtract }
    let re_module_exports = Regex::new(r"module\.exports\s*=\s*\{([^}]*)\}").unwrap();
    if let Some(caps) = re_module_exports.captures(source) {
        let names = caps[1].to_string();
        for part in names.split(',') {
            let name = part.trim();
            // Handle shorthand: { add, subtract } or named: { add: fn(){} }
            if !name.is_empty() {
                let export_name = name.split(':').next().unwrap_or(name).trim().to_string();
                if !export_name.is_empty() {
                    exports.insert(export_name);
                }
            }
        }
    }

    // Named exports: export { a, b }
    let re_named_exports = Regex::new(r"export\s+\{([^}]*)\}").unwrap();
    if let Some(caps) = re_named_exports.captures(source) {
        let names = caps[1].to_string();
        for part in names.split(',') {
            let name = part.trim();
            if !name.is_empty() {
                exports.insert(name.to_string());
            }
        }
    }

    let mut result: Vec<String> = exports.into_iter().collect();
    result.sort();
    result
}

fn extract_functions(source: &str, exported: &[String]) -> Vec<FunctionDecl> {
    let mut functions = Vec::new();

    // Function declarations: function name() or async function name()
    let re_fn_decl = Regex::new(r"(?:async\s+)?function\s+(\w+)\s*\(").unwrap();
    for caps in re_fn_decl.captures_iter(source) {
        if let Some(name) = caps.get(1) {
            let is_async = caps[0].starts_with("async");
            let is_exported = exported.contains(&name.as_str().to_string());
            functions.push(FunctionDecl {
                name: name.as_str().to_string(),
                is_async,
                is_exported,
            });
        }
    }

    // Arrow functions assigned to const/let/var: const name = () => or async () =>
    let re_arrow = Regex::new(r"(?:const|let|var)\s+(\w+)\s*=\s*(?:async\s+)?\(").unwrap();
    for caps in re_arrow.captures_iter(source) {
        if let Some(name) = caps.get(1) {
            let name_str = name.as_str().to_string();
            // Avoid duplicates with function declarations
            if !functions.iter().any(|f| f.name == name_str) {
                functions.push(FunctionDecl {
                    name: name_str,
                    is_async: false,
                    is_exported: exported.contains(&name.as_str().to_string()),
                });
            }
        }
    }

    functions.sort_by(|a, b| a.name.cmp(&b.name));
    functions
}

fn extract_classes(source: &str) -> Vec<ClassDecl> {
    let mut classes = Vec::new();

    // Find class declarations - look for class name and extends clause
    let re_class_header = Regex::new(r"class\s+(\w+)(?:\s+extends\s+(\w+))?\s*\{").unwrap();
    
    for caps in re_class_header.captures_iter(source) {
        if let Some(name_cap) = caps.get(1) {
            let name = name_cap.as_str().to_string();
            let extends = caps.get(2).map(|m| m.as_str().to_string());

            // Extract methods by finding all method-like patterns until we hit another class or end
            let remaining = &source[caps.get(0).unwrap().start()..];
            let methods = extract_methods_for_class(remaining);

            classes.push(ClassDecl {
                name,
                extends,
                methods,
            });
        }
    }

    classes.sort_by(|a, b| a.name.cmp(&b.name));
    classes
}

fn extract_methods_for_class(source: &str) -> Vec<String> {
    let mut methods = Vec::new();
    
    // Look for method patterns: name() { or async name() {
    // Stop when we hit another class declaration or end of input
    let re_method = Regex::new(r"(?:async\s+)?(\w+)\s*\([^)]*\)\s*\{").unwrap();
    
    // Find all method candidates
    for caps in re_method.captures_iter(source) {
        if let Some(name) = caps.get(1) {
            let name_str = name.as_str().to_string();
            // Skip constructor and keywords that aren't methods
            if name_str != "constructor" && !["if", "else", "for", "while", "switch"].contains(&name_str.as_str()) {
                methods.push(name_str);
            }
        }
    }

    methods.sort();
    methods.dedup();
    methods
}

fn detect_globals(source: &str) -> Vec<String> {
    let mut globals = HashSet::new();

    // Check for access to known global objects
    let global_names = [
        "window", "document", "navigator", "location", "history",
        "screen", "localStorage", "sessionStorage", "indexedDB",
        "performance", "crypto", "setTimeout", "setInterval",
        "clearTimeout", "clearInterval", "requestAnimationFrame",
        "fetch", "WebSocket", "Worker", "BroadcastChannel",
    ];

    for name in global_names {
        // Use word boundary to avoid matching substrings
        let re = Regex::new(&format!(r"\b{}\b", name)).unwrap();
        if re.is_match(source) {
            globals.insert(name.to_string());
        }
    }

    let mut result: Vec<String> = globals.into_iter().collect();
    result.sort();
    result
}

fn detect_dom_apis(source: &str) -> Vec<String> {
    let mut apis = HashSet::new();

    // Common DOM API patterns
    let dom_patterns = [
        ("getElementById", r"document\.getElementById"),
        ("getElementsByClassName", r"document\.getElementsByClassName"),
        ("querySelector", r"(?:document|element)\.querySelector"),
        ("querySelectorAll", r"(?:document|element)\.querySelectorAll"),
        ("addEventListener", r"\.addEventListener"),
        ("removeEventListener", r"\.removeEventListener"),
        ("createElement", r"document\.createElement"),
        ("innerHTML", r"\.innerHTML"),
        ("textContent", r"\.textContent"),
        ("classList", r"\.classList"),
        ("style", r"\.style\."),
    ];

    for (name, pattern) in dom_patterns {
        let re = Regex::new(pattern).unwrap();
        if re.is_match(source) {
            apis.insert(name.to_string());
        }
    }

    let mut result: Vec<String> = apis.into_iter().collect();
    result.sort();
    result
}

fn detect_network_calls(source: &str) -> Vec<NetworkCall> {
    let mut calls = Vec::new();

    // fetch() calls
    let re_fetch = Regex::new(r#"fetch\s*\(\s*['"]([^'"]+)['"]"#).unwrap();
    for caps in re_fetch.captures_iter(source) {
        if let Some(url) = caps.get(1) {
            calls.push(NetworkCall {
                kind: "fetch".to_string(),
                url_pattern: Some(url.as_str().to_string()),
            });
        }
    }

    // XMLHttpRequest
    let re_xhr = Regex::new(r"new\s+XMLHttpRequest").unwrap();
    if re_xhr.is_match(source) {
        calls.push(NetworkCall {
            kind: "XMLHttpRequest".to_string(),
            url_pattern: None,
        });
    }

    // WebSocket
    let re_ws = Regex::new(r#"new\s+WebSocket\s*\(\s*['"]([^'"]+)['"]"#).unwrap();
    for caps in re_ws.captures_iter(source) {
        if let Some(url) = caps.get(1) {
            calls.push(NetworkCall {
                kind: "WebSocket".to_string(),
                url_pattern: Some(url.as_str().to_string()),
            });
        }
    }

    calls
}

fn detect_obfuscation(source: &str) -> ObfuscationReport {
    let mut suspicious = Vec::new();

    // eval() usage
    let re_eval = Regex::new(r"\beval\s*\(").unwrap();
    let eval_usage = re_eval.is_match(source);
    if eval_usage {
        suspicious.push("eval() used".to_string());
    }

    // base64 decoding
    let re_b64 = Regex::new(r"atob|btoa|Buffer\.from\([^)]+,'base64'\)").unwrap();
    let base64_decoding = re_b64.is_match(source);
    if base64_decoding {
        suspicious.push("Base64 encoding/decoding detected".to_string());
    }

    // String concatenation (potential obfuscation)
    let re_concat = Regex::new(r#"['"][^'"]*['"]\s*\+\s*['"]"#).unwrap();
    let string_concatenation = re_concat.is_match(source);
    if string_concatenation {
        suspicious.push("String concatenation detected".to_string());
    }

    // Function constructor
    let re_fn_constructor = Regex::new(r"new\s+Function\s*\(").unwrap();
    if re_fn_constructor.is_match(source) {
        suspicious.push("Function() constructor used".to_string());
    }

    ObfuscationReport {
        eval_usage,
        base64_decoding,
        string_concatenation,
        suspicious_patterns: suspicious,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_test_file(content: &str) -> String {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.js");
        fs::write(&path, content).unwrap();
        // Leak the tempdir so it stays alive for the test duration
        let s = path.to_string_lossy().to_string();
        std::mem::forget(dir);
        s
    }

    #[test]
    fn test_extract_imports_es6() {
        let source = r#"import { foo, bar } from 'lodash';
import React from 'react';
import './style.css';"#;
        let imports = extract_imports(source);
        assert!(imports.contains(&"lodash".to_string()));
        assert!(imports.contains(&"react".to_string()));
        assert!(imports.contains(&"./style.css".to_string()));
    }

    #[test]
    fn test_extract_imports_commonjs() {
        let source = r#"const fs = require('fs');
const path = require("path");"#;
        let imports = extract_imports(source);
        assert!(imports.contains(&"fs".to_string()));
        assert!(imports.contains(&"path".to_string()));
    }

    #[test]
    fn test_extract_exports() {
        let source = r#"export function hello() {}
export class World {}
export const PI = 3.14;
module.exports = { add, subtract };
export { foo, bar };"#;
        let exports = extract_exports(source);
        assert!(exports.contains(&"hello".to_string()));
        assert!(exports.contains(&"World".to_string()));
        assert!(exports.contains(&"PI".to_string()));
        assert!(exports.contains(&"add".to_string()));
        assert!(exports.contains(&"subtract".to_string()));
        assert!(exports.contains(&"foo".to_string()));
        assert!(exports.contains(&"bar".to_string()));
    }

    #[test]
    fn test_extract_functions() {
        let source = r#"function regular() {}
async function asyncFn() {}
const arrow = () => {};
const namedArrow = (x) => x;"#;
        let exports = extract_exports(source);
        let functions = extract_functions(source, &exports);
        assert_eq!(functions.len(), 4);
        assert!(functions.iter().any(|f| f.name == "regular"));
        assert!(functions.iter().any(|f| f.name == "asyncFn" && f.is_async));
        assert!(functions.iter().any(|f| f.name == "arrow"));
        assert!(functions.iter().any(|f| f.name == "namedArrow"));
    }

    #[test]
    fn test_extract_classes() {
        let source = r#"class Animal {}
class Dog extends Animal {
    bark() {}
    sit() {}
}"#;
        let classes = extract_classes(source);
        assert_eq!(classes.len(), 2);
        let dog = classes.iter().find(|c| c.name == "Dog").unwrap();
        assert_eq!(dog.extends, Some("Animal".to_string()));
        assert!(dog.methods.iter().any(|m| m == "bark"));
        assert!(dog.methods.iter().any(|m| m == "sit"));
    }

    #[test]
    fn test_detect_globals() {
        let source = r#"window.alert('hi');
document.getElementById('x');
localStorage.setItem('k', 'v');"#;
        let globals = detect_globals(source);
        assert!(globals.contains(&"window".to_string()));
        assert!(globals.contains(&"document".to_string()));
        assert!(globals.contains(&"localStorage".to_string()));
    }

    #[test]
    fn test_detect_dom_apis() {
        let source = r#"document.getElementById('app');
element.addEventListener('click', handler);
div.innerHTML = 'hello';"#;
        let apis = detect_dom_apis(source);
        assert!(apis.contains(&"getElementById".to_string()));
        assert!(apis.contains(&"addEventListener".to_string()));
        assert!(apis.contains(&"innerHTML".to_string()));
    }

    #[test]
    fn test_detect_network_calls() {
        let source = r#"fetch('/api/users');
new XMLHttpRequest();
new WebSocket('ws://example.com');"#;
        let calls = detect_network_calls(source);
        assert_eq!(calls.len(), 3);
        assert!(calls.iter().any(|c| c.kind == "fetch"));
        assert!(calls.iter().any(|c| c.kind == "XMLHttpRequest"));
        assert!(calls.iter().any(|c| c.kind == "WebSocket"));
    }

    #[test]
    fn test_detect_obfuscation() {
        let source = r#"eval('alert(1)');
atob('SGVsbG8=');
'he' + 'llo';"#;
        let report = detect_obfuscation(source);
        assert!(report.eval_usage);
        assert!(report.base64_decoding);
        assert!(report.string_concatenation);
    }

    #[test]
    fn test_full_analysis() {
        let source = r#"import { map } from 'lodash';
import React from 'react';

export function processData(data) {
    return map(data, item => item * 2);
}

class Calculator {
    add(a, b) { return a + b; }
}

fetch('/api/data');
"#;
        let path = write_test_file(source);
        let result = analyze_js(&path).unwrap();
        assert!(result.data.imports.contains(&"lodash".to_string()));
        assert!(result.data.exports.contains(&"processData".to_string()));
        assert_eq!(result.data.classes.len(), 1);
        assert!(result.data.network_calls.iter().any(|c| c.kind == "fetch"));
    }
}