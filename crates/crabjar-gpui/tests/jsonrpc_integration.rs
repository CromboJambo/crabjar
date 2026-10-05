//! Integration test: spawn crabjar-terrarium-app in JSON-RPC mode, send commands, verify responses.

use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use serde_json::{json, Value};

async fn read_line(reader: &mut BufReader<tokio::process::ChildStdout>) -> Option<String> {
    let mut line = String::new();
    match reader.read_line(&mut line).await {
        Ok(0) => None,
        Ok(_) => Some(line.trim().to_string()),
        Err(_) => None,
    }
}

#[tokio::test]
async fn test_jsonrpc_ping() {
    // Spawn terrarium plugin in JSON-RPC mode (use pre-built binary)
    let mut child = tokio::process::Command::new("/home/crombo/projects/active/crabjar/target/debug/crabjar-terrarium-plugin")
        .arg("stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn terrarium");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // Send a valid command: start the terrarium
    let start_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "terrarium/start",
        "params": {
            "action": "start"
        }
    });

    stdin.write_all(start_req.to_string().as_bytes()).await.unwrap();
    stdin.write_all(b"\n").await.unwrap();
    stdin.flush().await.unwrap();

    // Wait for response (with timeout)
    let line = tokio::time::timeout(
        tokio::time::Duration::from_secs(10),
        read_line(&mut reader)
    ).await.expect("Timeout waiting for start response");

    let line = line.expect("No response received");
    let response: Value = serde_json::from_str(&line).expect("Invalid JSON");
    
    assert_eq!(response["id"], 1, "Response ID mismatch");
    assert!(response.get("result").is_some(), "Expected result field");
    eprintln!("Start response: {}", line);

    // Cleanup
    child.kill().await.unwrap();
}

#[tokio::test]
async fn test_jsonrpc_query_state() {
    // Spawn terrarium plugin in JSON-RPC mode (use pre-built binary)
    let mut child = tokio::process::Command::new("/home/crombo/projects/active/crabjar/target/debug/crabjar-terrarium-plugin")
        .arg("stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn terrarium");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // Send a valid command: pause the terrarium (doesn't require running)
    let query_req = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "terrarium/pause",
        "params": {
            "action": "pause"
        }
    });

    stdin.write_all(query_req.to_string().as_bytes()).await.unwrap();
    stdin.write_all(b"\n").await.unwrap();
    stdin.flush().await.unwrap();

    // Wait for response
    let line = tokio::time::timeout(
        tokio::time::Duration::from_secs(10),
        read_line(&mut reader)
    ).await.expect("Timeout waiting for query response");

    let line = line.expect("No response received");
    let response: Value = serde_json::from_str(&line).expect("Invalid JSON");
    
    assert_eq!(response["id"], 2, "Response ID mismatch");
    eprintln!("Query response: {}", line);

    // Cleanup
    child.kill().await.unwrap();
}