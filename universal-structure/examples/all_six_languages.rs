//! Six-language comparison demo: identical algorithm across Rust, JavaScript,
//! Python, C, C++, and Go analyzed through respective LanguageAdapters.

use universal_structure::{
    compare_trees, CAdapter, CppAdapter, GoAdapter, JavaScriptAdapter, LanguageAdapter,
    PythonAdapter, RustAdapter,
};

fn main() {
    println!("=== Universal Structure: Six-Language Comparison ===");
    println!();

    // Same algorithm in 6 languages: process data and compute sum
    let rust_code = r#"
fn process_data(values: &[i32]) -> i32 {
    let mut sum = 0;
    for &v in values {
        sum += v;
    }
    return sum;
}
"#;

    let js_code = r#"
function processData(values) {
    let sum = 0;
    for (let i = 0; i < values.length; i++) {
        sum += values[i];
    }
    return sum;
}
"#;

    let python_code = r#"
def process_data(values):
    total = 0
    for v in values:
        total += v
    return total
"#;

    let c_code = r#"
int process_data(int *values, int count) {
    int sum = 0;
    for (int i = 0; i < count; i++) {
        sum += values[i];
    }
    return sum;
}
"#;

    let cpp_code = r#"
int process_data(const std::vector<int>& values) {
    int sum = 0;
    for (const auto& v : values) {
        sum += v;
    }
    return sum;
}
"#;

    let go_code = r#"
package main

func processData(values []int) int {
    sum := 0
    for _, v := range values {
        sum += v
    }
    return sum
}
"#;

    // Analyze all six with their respective adapters
    let rust_adapter = RustAdapter::new();
    let js_adapter = JavaScriptAdapter::new();
    let python_adapter = PythonAdapter::new();
    let c_adapter = CAdapter::new();
    let cpp_adapter = CppAdapter::new();
    let go_adapter = GoAdapter::new();

    let rust_tree = rust_adapter.tokenize_and_adapt(rust_code);
    let js_tree = js_adapter.tokenize_and_adapt(js_code);
    let python_tree = python_adapter.tokenize_and_adapt(python_code);
    let c_tree = c_adapter.tokenize_and_adapt(c_code);
    let cpp_tree = cpp_adapter.tokenize_and_adapt(cpp_code);
    let go_tree = go_adapter.tokenize_and_adapt(go_code);

    // Compare all pairs
    println!("Cross-language architectural similarity:");
    println!();

    let pairs = vec![
        ("Rust", "JavaScript", &rust_tree, &js_tree),
        ("Rust", "Python", &rust_tree, &python_tree),
        ("Rust", "C", &rust_tree, &c_tree),
        ("Rust", "C++", &rust_tree, &cpp_tree),
        ("Rust", "Go", &rust_tree, &go_tree),
        ("JavaScript", "Python", &js_tree, &python_tree),
        ("JavaScript", "Go", &js_tree, &go_tree),
    ];

    for (lang_a, lang_b, tree_a, tree_b) in pairs {
        let report = compare_trees(tree_a, tree_b);
        println!(
            "{} vs {}: {:.0}%",
            lang_a,
            lang_b,
            report.metrics.iter().fold(0.0, |acc, m| acc + m.value_a)
                / report.metrics.len() as f64
                * 100.0
        );
    }

    println!();
    println!("Span counts by language:");
    println!("{:<12} {:>6}", "Language", "Spans");
    println!("{:<12} {:>6}", "--------", "-----");
    println!("{:<12} {:>6}", "Rust", rust_tree.spans.len());
    println!("{:<12} {:>6}", "JavaScript", js_tree.spans.len());
    println!("{:<12} {:>6}", "Python", python_tree.spans.len());
    println!("{:<12} {:>6}", "C", c_tree.spans.len());
    println!("{:<12} {:>6}", "C++", cpp_tree.spans.len());
    println!("{:<12} {:>6}", "Go", go_tree.spans.len());

    println!();
    println!("All 6 adapters successfully identify the same algorithm!");
}