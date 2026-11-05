use universal_structure::{JavaScriptAdapter, LanguageAdapter, RustAdapter, compare_trees};

fn main() {
    // Test Rust adapter with real-world-like code
    let rust_code = r#"
pub fn process_data(input: &[u8]) -> Result<String, Box<dyn Error>> {
    let mut result = String::new();
    for byte in input {
        if *byte > 127 {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "non-ASCII byte"
            )));
        }
        result.push(*byte as char);
    }
    Ok(result)
}

pub struct DataProcessor {
    buffer: Vec<u8>,
}

impl DataProcessor {
    pub fn new() -> Self {
        DataProcessor {
            buffer: Vec::new(),
        }
    }

    pub fn process(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        for byte in data {
            self.buffer.push(*byte);
        }
        Ok(())
    }
}
"#;

    let rust_adapter = RustAdapter::new();
    let rust_tree = rust_adapter.tokenize_and_adapt(rust_code);

    println!("Rust adapter results:");
    println!("  Spans identified: {}", rust_tree.spans.len());
    for span in &rust_tree.spans {
        println!("    - {:?}", span.kind);
    }
    println!("  Semantic tags:");
    for (key, value) in &rust_tree.semantic_tags {
        println!("    {} = {}", key, value);
    }

    // Compare with JavaScript equivalent
    let js_code = r#"
function processData(input) {
    let result = "";
    for (let i = 0; i < input.length; i++) {
        if (input[i] > 127) {
            throw new Error("non-ASCII byte");
        }
        result += String.fromCharCode(input[i]);
    }
    return result;
}

class DataProcessor {
    constructor() {
        this.buffer = [];
    }

    process(data) {
        for (let i = 0; i < data.length; i++) {
            this.buffer.push(data[i]);
        }
    }
}
"#;

    let js_adapter = JavaScriptAdapter::new();
    let js_tree = js_adapter.tokenize_and_adapt(js_code);

    println!("\nJavaScript adapter results:");
    println!("  Spans identified: {}", js_tree.spans.len());

    // Cross-language comparison
    let report = compare_trees(&rust_tree, &js_tree);
    println!("\nCross-language comparison metrics:");
    for metric in &report.metrics {
        println!("  {}:", metric.name);
        println!("    Rust: {:.2}", metric.value_a);
        println!("    JS:   {:.2}", metric.value_b);
        let diff = (metric.value_a - metric.value_b).abs();
        println!("    Diff: {:.2}%", diff * 100.0);
    }
}
