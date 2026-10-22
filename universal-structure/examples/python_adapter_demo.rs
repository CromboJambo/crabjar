//! Demonstration of the Python LanguageAdapter using pesti-structural-tokenizer.
//! Shows how Python code is tokenized, adapted to universal structural tokens,
//! and compared across languages with Rust and JavaScript equivalents.

use universal_structure::{compare_trees, LanguageAdapter, PythonAdapter};
use universal_structure::adapters::{JavaScriptAdapter, RustAdapter};

fn main() {
    println!("=== Python LanguageAdapter Demo ===\n");

    let py_adapter = PythonAdapter::new();
    let rust_adapter = RustAdapter::new();
    let js_adapter = JavaScriptAdapter::new();

    // Real-world-like Python code with modern features
    let python_code = r#"import asyncio
from typing import List, Optional

async def process_data(items: List[str]) -> List[str]:
    """Process items asynchronously."""
    results = []
    for item in items:
        if item.startswith("@"):
            continue
        processed = await transform(item)
        results.append(processed)
    return results

class DataProcessor:
    def __init__(self, config: dict):
        self.config = config
        self.cache = {}

    async def process(self, data: str) -> Optional[str]:
        try:
            if data in self.cache:
                return self.cache[data]
            result = await self._transform(data)
            self.cache[data] = result
            return result
        except Exception as e:
            print(f"Error: {e}")
            return None

    async def _transform(self, data: str) -> str:
        return data.upper()
"#;

    println!("--- Python Code Analysis ---");
    let py_tree = py_adapter.tokenize_and_adapt(python_code);
    println!("Language: {}", py_tree.language);
    println!("Spans identified: {}\n", py_tree.spans.len());

    for span in &py_tree.spans {
        println!("  {} → {}", span.kind.as_str(), span.source.trim());
    }

    if !py_tree.semantic_tags.is_empty() {
        println!("\nPython semantic tags:");
        for (key, value) in &py_tree.semantic_tags {
            println!("  {}: {}", key, value);
        }
    }

    // Rust equivalent
    let rust_code = r#"use std::collections::HashMap;
use async_trait::async_trait;

#[derive(Debug)]
pub struct DataProcessor {
    config: HashMap<String, String>,
    cache: HashMap<String, String>,
}

impl DataProcessor {
    pub fn new(config: HashMap<String, String>) -> Self {
        Self {
            config,
            cache: HashMap::new(),
        }
    }

    pub async fn process(&mut self, data: &str) -> Option<String> {
        if let Some(cached) = self.cache.get(data) {
            return Some(cached.clone());
        }
        match self.transform(data).await {
            Ok(result) => {
                self.cache.insert(data.to_string(), result.clone());
                Some(result)
            }
            Err(_) => None,
        }
    }

    async fn transform(&self, data: &str) -> Result<String, Box<dyn std::error::Error>> {
        Ok(data.to_uppercase())
    }
}
"#;

    println!("\n--- Rust Equivalent Analysis ---");
    let rust_tree = rust_adapter.tokenize_and_adapt(rust_code);
    println!("Language: {}", rust_tree.language);
    println!("Spans identified: {}\n", rust_tree.spans.len());

    for span in &rust_tree.spans {
        println!("  {} → {}", span.kind.as_str(), span.source.trim());
    }

    if !rust_tree.semantic_tags.is_empty() {
        println!("\nRust semantic tags:");
        for (key, value) in &rust_tree.semantic_tags {
            println!("  {}: {}", key, value);
        }
    }

    // JavaScript equivalent
    let js_code = r#"class DataProcessor {
    constructor(config) {
        this.config = config;
        this.cache = new Map();
    }

    async process(data) {
        try {
            if (this.cache.has(data)) {
                return this.cache.get(data);
            }
            const result = await this.transform(data);
            this.cache.set(data, result);
            return result;
        } catch (e) {
            console.error("Error:", e.message);
            return null;
        }
    }

    async transform(data) {
        return data.toUpperCase();
    }
}
"#;

    println!("\n--- JavaScript Equivalent Analysis ---");
    let js_tree = js_adapter.tokenize_and_adapt(js_code);
    println!("Language: {}", js_tree.language);
    println!("Spans identified: {}\n", js_tree.spans.len());

    for span in &js_tree.spans {
        println!("  {} → {}", span.kind.as_str(), span.source.trim());
    }

    if !js_tree.semantic_tags.is_empty() {
        println!("\nJavaScript semantic tags:");
        for (key, value) in &js_tree.semantic_tags {
            println!("  {}: {}", key, value);
        }
    }

    // Cross-language comparison
    println!("\n=== Cross-Language Comparison ===\n");

    let py_rust = compare_trees(&py_tree, &rust_tree);
    println!("Python vs Rust:");
    for metric in &py_rust.metrics {
        println!("  {}: {:.2} vs {:.2}", metric.name, metric.value_a, metric.value_b);
    }

    let py_js = compare_trees(&py_tree, &js_tree);
    println!("\nPython vs JavaScript:");
    for metric in &py_js.metrics {
        println!("  {}: {:.2} vs {:.2}", metric.name, metric.value_a, metric.value_b);
    }

    let rust_js = compare_trees(&rust_tree, &js_tree);
    println!("\nRust vs JavaScript:");
    for metric in &rust_js.metrics {
        println!("  {}: {:.2} vs {:.2}", metric.name, metric.value_a, metric.value_b);
    }

    println!("\n=== Demo Complete ===");
}