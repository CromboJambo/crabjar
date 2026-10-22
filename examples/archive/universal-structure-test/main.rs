//! Cross-language code comparison test using universal structural tokens.
//!
//! Analyzes real-world code samples and compares architectural properties.

use std::fs;
use universal_structure::{LanguageAdapter, SpanKind};
use universal_structure::adapters::{RustAdapter, JavaScriptAdapter};
use universal_structure::comparison::compare_trees;

fn analyze_rust_file(path: &str) -> Result<universal_structure::SpanTree, Box<dyn std::error::Error>> {
    let source = fs::read_to_string(path)?;
    let adapter = RustAdapter::new();
    Ok(adapter.tokenize_and_adapt(&source))
}

fn analyze_js_file(path: &str) -> Result<universal_structure::SpanTree, Box<dyn std::error::Error>> {
    let source = fs::read_to_string(path)?;
    let adapter = JavaScriptAdapter::new();
    Ok(adapter.tokenize_and_adapt(&source))
}

fn print_tree_stats(tree: &universal_structure::SpanTree) {
    println!("Language: {}", tree.language);
    println!("Source: {}", tree.source);
    
    // Count various span kinds
    let fn_count = tree.spans.iter().filter(|s| s.kind == SpanKind::FnDecl).count();
    let class_count = tree.spans.iter().filter(|s| s.kind == SpanKind::ClassDecl).count();
    let error_handling = tree.spans.iter().filter(|s| s.kind.is_error_handling()).count();
    let control_flow = tree.spans.iter().filter(|s| s.kind.is_control_flow()).count();
    
    println!("Functions: {}", fn_count);
    println!("Classes: {}", class_count);
    println!("Error handling patterns: {}", error_handling);
    println!("Control flow constructs: {}", control_flow);
}

fn main() {
    println!("=== Universal Structure Cross-Language Analysis ===\n");
    
    // Test with crabjar's own Rust code
    println!("--- Analyzing crabjar guard module (Rust) ---");
    match analyze_rust_file("../guard/src/lib.rs") {
        Ok(tree) => {
            print_tree_stats(&tree);
            
            // Compare with a JavaScript equivalent pattern
            let js_code = r#"
// Equivalent error handling in JavaScript
async function processWithRetry(fn, retries = 3) {
    for (let i = 0; i < retries; i++) {
        try {
            return await fn();
        } catch (error) {
            if (i === retries - 1) throw error;
            console.warn('Retrying...', error.message);
        }
    }
}

async function riskyOperation() {
    const result = await fetch('/api/data');
    if (!result.ok) throw new Error('HTTP ' + result.status);
    return result.json();
}
"#;
            
            let js_adapter = JavaScriptAdapter::new();
            let mut js_tree = js_adapter.tokenize_and_adapt(js_code);
            js_tree.source = "js_error_handling.js".to_string();
            
            println!("\n--- Analyzing JavaScript error handling pattern ---");
            print_tree_stats(&js_tree);
            
            println!("\n--- Cross-language comparison ---");
            let report = compare_trees(&tree, &js_tree);
            println!("{}", report.summary());
        }
        Err(e) => println!("Error analyzing Rust: {}", e),
    }
    
    // Test with a real JavaScript project pattern
    println!("\n--- Analyzing Express.js-style middleware (JavaScript) ---");
    let express_code = r#"
const express = require('express');
const app = express();

function logger(req, res, next) {
    console.log(`${req.method} ${req.url}`);
    next();
}

function errorHandler(err, req, res, next) {
    console.error(err.stack);
    res.status(500).json({ error: err.message });
}

app.use(logger);
app.use(express.json());

app.get('/api/users', async (req, res) => {
    try {
        const users = await User.findAll();
        res.json(users);
    } catch (error) {
        next(error);
    }
});

app.use(errorHandler);
"#;

    let js_adapter = JavaScriptAdapter::new();
    let mut express_tree = js_adapter.tokenize_and_adapt(express_code);
    express_tree.source = "express_middleware.js".to_string();
    
    print_tree_stats(&express_tree);
    
    // Compare Express pattern with crabjar's architecture
    println!("\n--- Comparing Express middleware vs crabjar guard ---");
    match analyze_rust_file("../guard/src/lib.rs") {
        Ok(rust_tree) => {
            let report = compare_trees(&rust_tree, &express_tree);
            println!("{}", report.summary());
        }
        Err(e) => println!("Error analyzing Rust: {}", e),
    }
}
