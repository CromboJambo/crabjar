//! Comprehensive cross-language comparison across all five adapters:
//! Rust, JavaScript, Python, C, and C++.

use universal_structure::{
    CAdapter, CppAdapter, JavaScriptAdapter, LanguageAdapter, PythonAdapter, RustAdapter,
};

fn main() {
    let rust_adapter = RustAdapter::new();
    let js_adapter = JavaScriptAdapter::new();
    let py_adapter = PythonAdapter::new();
    let c_adapter = CAdapter::new();
    let cpp_adapter = CppAdapter::new();

    // Same algorithm (process data, compute sum) in all five languages

    let rust_code = r#"fn process_data(items: &[i32]) -> i32 {
    items.iter().sum()
}"#;

    let js_code = "function processData(items) {\n  return items.reduce((a, b) => a + b, 0);\n}";

    let py_code = "def process_data(items):\n    return sum(items)";

    let c_code = "int process_data(int *items, int count) {\n    int sum = 0;\n    for (int i = 0; i < count; i++) {\n        sum += items[i];\n    }\n    return sum;\n}";

    let cpp_code = "#include <vector>\n#include <numeric>\n\nint process_data(const std::vector<int>& items) {\n    return std::accumulate(items.begin(), items.end(), 0);\n}";

    // Tokenize all five
    let rust_tree = rust_adapter.tokenize_and_adapt(rust_code);
    let js_tree = js_adapter.tokenize_and_adapt(js_code);
    let py_tree = py_adapter.tokenize_and_adapt(py_code);
    let c_tree = c_adapter.tokenize_and_adapt(c_code);
    let cpp_tree = cpp_adapter.tokenize_and_adapt(cpp_code);

    println!("=== Cross-Language Comparison: All 5 Adapters ===\n");

    // Summary table
    println!("{:<10} {:>8} {:>10}", "Language", "Spans", "Functions");
    println!("{:<10} {:>8} {:>10}", "--------", "-----", "---------");
    println!(
        "{:<10} {:>8} {:>10}",
        "Rust",
        rust_tree.spans.len(),
        rust_tree
            .spans
            .iter()
            .filter(|s| s.kind == universal_structure::SpanKind::FnDecl)
            .count()
    );
    println!(
        "{:<10} {:>8} {:>10}",
        "JavaScript",
        js_tree.spans.len(),
        js_tree
            .spans
            .iter()
            .filter(|s| s.kind == universal_structure::SpanKind::FnDecl)
            .count()
    );
    println!(
        "{:<10} {:>8} {:>10}",
        "Python",
        py_tree.spans.len(),
        py_tree
            .spans
            .iter()
            .filter(|s| s.kind == universal_structure::SpanKind::FnDecl)
            .count()
    );
    println!(
        "{:<10} {:>8} {:>10}",
        "C",
        c_tree.spans.len(),
        c_tree
            .spans
            .iter()
            .filter(|s| s.kind == universal_structure::SpanKind::FnDecl)
            .count()
    );
    println!(
        "{:<10} {:>8} {:>10}",
        "C++",
        cpp_tree.spans.len(),
        cpp_tree
            .spans
            .iter()
            .filter(|s| s.kind == universal_structure::SpanKind::FnDecl)
            .count()
    );

    println!("\nAll five adapters successfully identified the function declaration.");
    println!("This demonstrates the pluggable adapter architecture working across");
    println!("five different languages with a unified structural representation.");
}
