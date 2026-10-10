// Unit tests for analyze-mcp-server tool handlers
use std::env;
use std::fs;
use std::path::PathBuf;

fn create_test_elf() -> PathBuf {
    let temp_dir = env::temp_dir();
    let test_file = temp_dir.join("analyze_mcp_test_elf");
    
    // Write minimal ELF header (magic bytes + some zeros)
    let mut f = fs::File::create(&test_file).unwrap();
    let mut elf_header = vec![0u8; 64];
    elf_header[0] = 0x7f;
    elf_header[1] = b'E';
    elf_header[2] = b'L';
    elf_header[3] = b'F';
    elf_header[4] = 2; // 64-bit
    elf_header[5] = 1; // Little endian
    elf_header[6] = 1; // ELF version
    f.write_all(&elf_header).unwrap();
    
    test_file
}

#[test]
fn analyze_binary_tool_exists_and_handles_path() {
    let server = analyze_mcp_server::AnalyzeMcpServer;
    
    let params = rmcp::handler::server::wrapper::Parameters(
        analyze_mcp_server::BinaryRequest { path: "/nonexistent/path".to_string() }
    );
    
    // Should return an error string for nonexistent file, not panic
    let result = server.analyze_binary(params);
    assert!(result.contains("Analysis error") || result.contains("error"));
}

#[test]
fn analyze_har_tool_exists() {
    let server = analyze_mcp_server::AnalyzeMcpServer;
    
    let params = rmcp::handler::server::wrapper::Parameters(
        analyze_mcp_server::BinaryRequest { path: "/nonexistent.har".to_string() }
    );
    
    let result = server.analyze_har(params);
    assert!(!result.is_empty());
}

#[test]
fn analyze_js_tool_exists() {
    let server = analyze_mcp_server::AnalyzeMcpServer;
    
    let params = rmcp::handler::server::wrapper::Parameters(
        analyze_mcp_server::BinaryRequest { path: "/nonexistent.js".to_string() }
    );
    
    let result = server.analyze_js(params);
    assert!(!result.is_empty());
}

#[test]
fn analyze_apk_tool_exists() {
    let server = analyze_mcp_server::AnalyzeMcpServer;
    
    let params = rmcp::handler::server::wrapper::Parameters(
        analyze_mcp_server::BinaryRequest { path: "/nonexistent.apk".to_string() }
    );
    
    let result = server.analyze_apk(params);
    assert!(!result.is_empty());
}

#[test]
fn analyze_firmware_tool_exists() {
    let server = analyze_mcp_server::AnalyzeMcpServer;
    
    let params = rmcp::handler::server::wrapper::Parameters(
        analyze_mcp_server::BinaryRequest { path: "/nonexistent.bin".to_string() }
    );
    
    let result = server.analyze_firmware(params);
    assert!(!result.is_empty());
}

#[test]
fn analyze_dotnet_tool_exists() {
    let server = analyze_mcp_server::AnalyzeMcpServer;
    
    let params = rmcp::handler::server::wrapper::Parameters(
        analyze_mcp_server::BinaryRequest { path: "/nonexistent.dll".to_string() }
    );
    
    let result = server.analyze_dotnet(params);
    assert!(!result.is_empty());
}

#[test]
fn binary_request_struct_serializes() {
    let req = analyze_mcp_server::BinaryRequest { path: "/tmp/test".to_string() };
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("/tmp/test"));
}