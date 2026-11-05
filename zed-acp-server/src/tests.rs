//! Tests for ACP server functionality.

use crate::{AcpAgentServer, handlers::AcpServerError};
use crabjar_guard::GuardDb;
use serde_json::json;
use tempfile::tempdir;

#[tokio::test]
async fn test_initialize() {
    let mut server = AcpAgentServer::new();
    let response = server.handle_request(crate::protocol::AcpRequest::Initialize {
        id: json!(1),
        protocol_version: 1,
        client_capabilities: json!({}),
        client_info: Some(json!({"name": "test", "version": "1.0"})),
    }).await;
    match response {
        Ok(crate::protocol::AcpResponse::Result { id, result }) => {
            assert_eq!(id, json!(1));
            assert_eq!(result["protocolVersion"], json!(1));
            assert_eq!(result["agentInfo"]["name"], json!("crabjar-acp-server"));
        }
        Ok(_) => panic!("unexpected error response"),
        Err(_) => panic!("expected result"),
    }
}

#[tokio::test]
async fn test_session_new() {
    let mut server = AcpAgentServer::new();
    let response = server.handle_request(crate::protocol::AcpRequest::SessionNew {
        id: json!(2),
        cwd: "/test/project".to_string(),
        mcp_servers: vec![],
    }).await;
    match response {
        Ok(crate::protocol::AcpResponse::Result { id, result }) => {
            assert_eq!(id, json!(2));
            assert!(result["sessionId"].is_string());
        }
        Ok(_) => panic!("unexpected error response"),
        Err(_) => panic!("expected result"),
    }
}

#[tokio::test]
async fn test_session_prompt() {
    let mut server = AcpAgentServer::new();
    // Create session first
    let new_response = server.handle_request(crate::protocol::AcpRequest::SessionNew {
        id: json!(1),
        cwd: "/test/project".to_string(),
        mcp_servers: vec![],
    }).await;
    let session_id = match new_response {
        Ok(crate::protocol::AcpResponse::Result { result, .. }) => {
            result["sessionId"].as_str().unwrap().to_string()
        }
        _ => panic!("failed to create session"),
    };

    // Now send prompt
    let response = server.handle_request(crate::protocol::AcpRequest::SessionPrompt {
        id: json!(2),
        session_id,
        prompt: vec![json!({"type": "text", "text": "test prompt"})],
    }).await;
    match response {
        Ok(crate::protocol::AcpResponse::Result { result, .. }) => {
            assert_eq!(result["status"], json!("processed"));
            assert_eq!(result["stopReason"], json!("end_turn"));
        }
        Ok(_) => panic!("unexpected error response"),
        Err(_) => panic!("expected result"),
    }
}

#[test]
fn test_tool_call_with_guard() {
    let dir = tempdir().unwrap();
    let db = GuardDb::open(dir.path().join("guard.db")).unwrap();
    let mut server = AcpAgentServer::new().with_guard_db(db);
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Create session first
    let new_response = rt.block_on(server.handle_request(crate::protocol::AcpRequest::SessionNew {
        id: json!(1),
        cwd: dir.path().to_string_lossy().into_owned(),
        mcp_servers: vec![],
    }));
    let session_id = match new_response {
        Ok(crate::protocol::AcpResponse::Result { result, .. }) => {
            result["sessionId"].as_str().unwrap().to_string()
        }
        _ => panic!("failed to create session"),
    };

    let response = rt.block_on(server.handle_request(crate::protocol::AcpRequest::ToolCall {
        id: json!(2),
        session_id,
        function_name: "echo".to_string(),
        arguments: json!({
            "args": ["hello"],
            "confidence": 0.9,
        }),
    }));
    match response {
        Ok(crate::protocol::AcpResponse::Result { result, .. }) => {
            // Gate may return "proceed", "interrupted", or "dry_run" depending on guard config
            let gate_result = result["gate_result"].as_str().unwrap();
            assert!(["proceed", "interrupted", "dry_run"].contains(&gate_result));
        }
        Ok(_) => panic!("unexpected error response"),
        Err(_) => panic!("expected result"),
    }
}

#[test]
fn test_tool_call_without_guard_db() {
    let mut server = AcpAgentServer::new();
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Create session first
    let new_response = rt.block_on(server.handle_request(crate::protocol::AcpRequest::SessionNew {
        id: json!(1),
        cwd: "/test".to_string(),
        mcp_servers: vec![],
    }));
    let session_id = match new_response {
        Ok(crate::protocol::AcpResponse::Result { result, .. }) => {
            result["sessionId"].as_str().unwrap().to_string()
        }
        _ => panic!("failed to create session"),
    };

    let response = rt.block_on(server.handle_request(crate::protocol::AcpRequest::ToolCall {
        id: json!(2),
        session_id,
        function_name: "echo".to_string(),
        arguments: json!({"args": []}),
    }));
    match response {
        Ok(crate::protocol::AcpResponse::Error { message, .. }) => {
            assert!(message.contains("guard"));
        }
        Err(AcpServerError::GuardError(msg)) => {
            assert!(msg.contains("guard"));
        }
        Ok(_) => panic!("expected error response"),
        Err(_) => panic!("expected ok with error result"),
    }
}
