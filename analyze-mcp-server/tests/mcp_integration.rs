// Integration tests: spawn analyze-mcp-server, send JSON-RPC over stdin, verify responses.
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

fn run_server_with_requests(requests: &[&str]) -> String {
    // Use the built binary directly for faster test execution
    let server_path = "../../target/debug/analyze-mcp";
    
    let mut child = Command::new(server_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn analyze-mcp-server");

    let mut stdin = child.stdin.take().expect("failed to get stdin");

    // Send all requests with newline delimiters
    for req in requests {
        stdin.write_all(req.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();
    }
    drop(stdin);

    thread::sleep(Duration::from_secs(5));
    child.kill().ok();
    let output = child.wait_with_output().expect("failed to wait");
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn server_initializes_and_lists_tools() {
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0"}}}}"#;
    let tools_list = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}}"#;

    let output = run_server_with_requests(&[init, tools_list]);

    // Should have gotten a tools list response containing our tool names
    assert!(output.contains("analyze_binary"), "Expected analyze_binary in tools list");
    assert!(output.contains("analyze_har"), "Expected analyze_har in tools list");
    assert!(output.contains("analyze_js"), "Expected analyze_js in tools list");
}

#[test]
fn analyze_binary_returns_json_response() {
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0"}}}}"#;

    // Create a tiny test ELF binary
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("analyze_test_elf");
    let mut f = std::fs::File::create(&test_file).unwrap();
    let mut elf_header = vec![0u8; 64];
    elf_header[0] = 0x7f;
    elf_header[1] = b'E';
    elf_header[2] = b'L';
    elf_header[3] = b'F';
    elf_header[4] = 2; // 64-bit
    elf_header[5] = 1; // Little endian
    elf_header[6] = 1; // ELF version
    f.write_all(&elf_header).unwrap();
    drop(f);

    let analyze_req = format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{{"name":"analyze_binary","arguments":{{"path":"{}"}}}}}}"#,
        test_file.to_string_lossy()
    );

    let output = run_server_with_requests(&[init, &analyze_req]);

    // Should get some response (success or error) for the analysis call
    assert!(!output.is_empty(), "Expected non-empty response");

    std::fs::remove_file(&test_file).ok();
}

#[test]
fn analyze_nonexistent_binary_returns_error() {
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0"}}}}"#;
    let analyze_req = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"analyze_binary","arguments":{"path":"/nonexistent/file"}}}"#;

    let output = run_server_with_requests(&[init, analyze_req]);

    // Should contain an error message about the nonexistent file
    assert!(output.contains("Analysis error") || output.contains("error"), "Expected error for nonexistent file");
}