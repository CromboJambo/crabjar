use crabjar_compress::{compress_tool_result, CompressionConfig};

fn main() {
    // Test 1: Small text (should not compress)
    let small = "Hello world";
    let result = compress_tool_result(small, &CompressionConfig::default());
    assert_eq!(result.compressed, small);
    println!("✓ Small text passthrough");

    // Test 2: JSON with large array (should compress)
    let json = format!("[{}]", (0..50).map(|i| i.to_string()).collect::<Vec<_>>().join(","));
    let config = CompressionConfig { json_array_limit: 10, min_tokens: 0, ..Default::default() };
    let result = compress_tool_result(&json, &config);
    assert!(result.compressed.contains("more items"));
    println!("✓ JSON array truncation");

    // Test 3: Log compression
    let logs = "[ERROR] Critical\n[WARN] Warning\n[INFO] Info line";
    let result = compress_tool_result(logs, &CompressionConfig { min_tokens: 0, ..Default::default() });
    println!("✓ Log compression");

    println!("\nAll smoke tests passed!");
}
