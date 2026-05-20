#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crabjar_guard::GuardDb;
    use crabjar_guard::GateResult;
    use crabjar_guard::ActionStatus;

    #[test]
    fn zed_request_to_acp_new_session_works() {
        let req = ZedRequest {
            method: "new_session".to_string(),
            params: serde_json::json!({ "cwd": "/tmp/test" }),
        };
        let acp = req.to_acp_request().unwrap();
        assert!(matches!(acp, AcpRequest::NewSession { cwd } if cwd == "/tmp/test"));
    }

    #[test]
    fn zed_request_to_acp_load_session_works() {
        let req = ZedRequest {
            method: "load_session".to_string(),
            params: serde_json::json!({
                "session_id": "abc",
                "cwd": "/tmp/test"
            }),
        };
        let acp = req.to_acp_request().unwrap();
        assert!(matches!(acp, AcpRequest::LoadSession { session_id, cwd } if session_id == "abc" && cwd == "/tmp/test"));
    }

    #[test]
    fn zed_request_to_acp_close_session_works() {
        let req = ZedRequest {
            method: "close_session".to_string(),
            params: serde_json::json!({ "session_id": "abc" }),
        };
        let acp = req.to_acp_request().unwrap();
        assert!(matches!(acp, AcpRequest::CloseSession { session_id } if session_id == "abc"));
    }

    #[test]
    fn zed_request_to_acp_list_sessions_works() {
        let req = ZedRequest {
            method: "list_sessions".to_string(),
            params: serde_json::json!({}),
        };
        let acp = req.to_acp_request().unwrap();
        assert!(matches!(acp, AcpRequest::ListSessions));
    }

    #[test]
    fn zed_request_to_acp_prompt_works() {
        let req = ZedRequest {
            method: "prompt".to_string(),
            params: serde_json::json!({
                "session_id": "abc",
                "message": "hello"
            }),
        };
        let acp = req.to_acp_request().unwrap();
        assert!(matches!(acp, AcpRequest::Prompt { session_id, message } if session_id == "abc" && message == "hello"));
    }

    #[test]
    fn zed_request_to_acp_tool_call_works() {
        let req = ZedRequest {
            method: "tool_call".to_string(),
            params: serde_json::json!({
                "session_id": "abc",
                "function_name": "run_command",
                "arguments": "[\"echo\", \"hello\"]"
            }),
        };
        let acp = req.to_acp_request().unwrap();
        assert!(matches!(acp, AcpRequest::ToolCall { session_id, function_name, arguments } if session_id == "abc" && function_name == "run_command" && arguments == "[\"echo\", \"hello\"]"));
    }

    #[test]
    fn zed_request_to_acp_authenticate_works() {
        let req = ZedRequest {
            method: "authenticate".to_string(),
            params: serde_json::json!({ "auth_method": "token" }),
        };
        let acp = req.to_acp_request().unwrap();
        assert!(matches!(acp, AcpRequest::Authenticate { auth_method } if auth_method == "token"));
    }

    #[test]
    fn zed_request_to_acp_unknown_method_errors() {
        let req = ZedRequest {
            method: "unknown_method".to_string(),
            params: serde_json::json!({}),
        };
        let result = req.to_acp_request();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown method"));
    }

    #[test]
    fn zed_request_to_acp_missing_cwd_errors() {
        let req = ZedRequest {
            method: "new_session".to_string(),
            params: serde_json::json!({}),
        };
        let result = req.to_acp_request();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cwd not found"));
    }

    #[test]
    fn zed_request_to_acp_missing_session_id_errors() {
        let req = ZedRequest {
            method: "load_session".to_string(),
            params: serde_json::json!({ "cwd": "/tmp" }),
        };
        let result = req.to_acp_request();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("session_id not found"));
    }

    #[test]
    fn zed_request_to_acp_missing_message_errors() {
        let req = ZedRequest {
            method: "prompt".to_string(),
            params: serde_json::json!({ "session_id": "abc" }),
        };
        let result = req.to_acp_request();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("message not found"));
    }

    #[test]
    fn zed_request_to_acp_missing_function_name_errors() {
        let req = ZedRequest {
            method: "tool_call".to_string(),
            params: serde_json::json!({ "session_id": "abc", "arguments": "[]" }),
        };
        let result = req.to_acp_request();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("function_name not found"));
    }

    #[test]
    fn zed_request_to_acp_missing_arguments_errors() {
        let req = ZedRequest {
            method: "tool_call".to_string(),
            params: serde_json::json!({ "session_id": "abc", "function_name": "run_command" }),
        };
        let result = req.to_acp_request();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("arguments not found"));
    }

    #[test]
    fn zed_request_to_acp_missing_auth_method_errors() {
        let req = ZedRequest {
            method: "authenticate".to_string(),
            params: serde_json::json!({}),
        };
        let result = req.to_acp_request();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("auth_method not found"));
    }

    #[test]
    fn acp_session_new_creates_session() {
        let session = AcpSession::new("/tmp/test".to_string());
        assert!(!session.id.is_empty());
        assert_eq!(session.cwd, "/tmp/test");
        assert_eq!(session.trust_layer, 2);
        assert!(session.trajectory.is_empty());
    }

    #[test]
    fn acp_bridge_new_session_works() {
        let dir = tempdir().unwrap();
        let guard_db = GuardDb::open(dir.path().join("guard.db")).unwrap();
        let bridge = AcpBridge {
            sessions: std::sync::Mutex::new(Vec::new()),
            guard_db,
        };
        let session = bridge.new_session("/tmp/test".to_string()).unwrap();
        assert!(!session.id.is_empty());
        assert_eq!(session.cwd, "/tmp/test");
    }

    #[test]
    fn acp_bridge_load_session_works() {
        let dir = tempdir().unwrap();
        let guard_db = GuardDb::open(dir.path().join("guard.db")).unwrap();
        let mut bridge = AcpBridge {
            sessions: std::sync::Mutex::new(Vec::new()),
            guard_db,
        };
        let session = bridge.new_session("/tmp/test".to_string()).unwrap();
        let loaded = bridge.load_session(session.id.clone(), "/tmp/new".to_string()).unwrap();
        assert_eq!(loaded.cwd, "/tmp/new");
    }

    #[test]
    fn acp_bridge_load_session_not_found_errors() {
        let mut bridge = AcpBridge {
            sessions: std::sync::Mutex::new(Vec::new()),
            guard_db: GuardDb::open(":memory:").unwrap(),
        };
        let result = bridge.load_session("nonexistent".to_string(), "/tmp".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn acp_bridge_close_session_works() {
        let dir = tempdir().unwrap();
        let guard_db = GuardDb::open(dir.path().join("guard.db")).unwrap();
        let mut bridge = AcpBridge {
            sessions: std::sync::Mutex::new(Vec::new()),
            guard_db,
        };
        let session = bridge.new_session("/tmp/test".to_string()).unwrap();
        bridge.close_session(session.id.clone()).unwrap();
        assert!(bridge.list_sessions().is_empty());
    }

    #[test]
    fn acp_bridge_close_session_nonexistent_noop() {
        let mut bridge = AcpBridge {
            sessions: std::sync::Mutex::new(Vec::new()),
            guard_db: GuardDb::open(":memory:").unwrap(),
        };
        bridge.close_session("nonexistent".to_string()).unwrap();
        assert!(bridge.list_sessions().is_empty());
    }

    #[test]
    fn acp_bridge_list_sessions_returns_all() {
        let dir = tempdir().unwrap();
        let guard_db = GuardDb::open(dir.path().join("guard.db")).unwrap();
        let mut bridge = AcpBridge {
            sessions: std::sync::Mutex::new(Vec::new()),
            guard_db,
        };
        bridge.new_session("/tmp/a".to_string()).unwrap();
        bridge.new_session("/tmp/b".to_string()).unwrap();
        let sessions = bridge.list_sessions();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn acp_bridge_record_event_works() {
        let dir = tempdir().unwrap();
        let guard_db = GuardDb::open(dir.path().join("guard.db")).unwrap();
        let mut bridge = AcpBridge {
            sessions: std::sync::Mutex::new(Vec::new()),
            guard_db,
        };
        let session = bridge.new_session("/tmp/test".to_string()).unwrap();
        let event = TrajectoryEvent {
            timestamp: 123,
            source: "user".to_string(),
            content: "hello".to_string(),
            preview: "hello".to_string(),
        };
        bridge.record_event(session.id.clone(), event).unwrap();
        assert_eq!(session.trajectory.len(), 1);
    }

    #[test]
    fn acp_bridge_record_event_not_found_errors() {
        let mut bridge = AcpBridge {
            sessions: std::sync::Mutex::new(Vec::new()),
            guard_db: GuardDb::open(":memory:").unwrap(),
        };
        let event = TrajectoryEvent {
            timestamp: 123,
            source: "user".to_string(),
            content: "hello".to_string(),
            preview: "hello".to_string(),
        };
        let result = bridge.record_event("nonexistent".to_string(), event);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn acp_bridge_gate_check_proceeds_for_default() {
        let dir = tempdir().unwrap();
        let guard_db = GuardDb::open(dir.path().join("guard.db")).unwrap();
        let mut bridge = AcpBridge {
            sessions: std::sync::Mutex::new(Vec::new()),
            guard_db,
        };
        let session = bridge.new_session("/tmp".to_string()).unwrap();
        let (gate_result, status) = bridge.gate_check(session.id.clone(), "echo", &["hello"]).unwrap();
        assert!(matches!(gate_result, GateResult::Proceed));
        assert!(matches!(status, ActionStatus::TrustApproved));
    }

    #[test]
    fn acp_bridge_gate_check_session_not_found_errors() {
        let mut bridge = AcpBridge {
            sessions: std::sync::Mutex::new(Vec::new()),
            guard_db: GuardDb::open(":memory:").unwrap(),
        };
        let result = bridge.gate_check("nonexistent".to_string(), "echo", &["hello"]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn handle_request_new_session_returns_result() {
        let mut server = AcpAgentServer::new();
        let req = ZedRequest {
            method: "new_session".to_string(),
            params: serde_json::json!({ "cwd": "/tmp/test" }),
        };
        let response = tokio::runtime::Runtime::new().unwrap().block_on(server.handle_request(req));
        assert!(matches!(response, AcpResponse::Result { .. }));
    }

    #[test]
    fn handle_request_close_session_returns_result() {
        let mut server = AcpAgentServer::new();
        let req = ZedRequest {
            method: "new_session".to_string(),
            params: serde_json::json!({ "cwd": "/tmp/test" }),
        };
        let _session = tokio::runtime::Runtime::new().unwrap().block_on(server.handle_request(req));
        let req2 = ZedRequest {
            method: "close_session".to_string(),
            params: serde_json::json!({ "session_id": "nonexistent" }),
        };
        let response = tokio::runtime::Runtime::new().unwrap().block_on(server.handle_request(req2));
        assert!(matches!(response, AcpResponse::Result { .. }));
    }

    #[test]
    fn handle_request_list_sessions_returns_result() {
        let mut server = AcpAgentServer::new();
        let req = ZedRequest {
            method: "list_sessions".to_string(),
            params: serde_json::json!({}),
        };
        let response = tokio::runtime::Runtime::new().unwrap().block_on(server.handle_request(req));
        assert!(matches!(response, AcpResponse::Result { .. }));
    }

    #[test]
    fn handle_request_unknown_method_returns_error() {
        let mut server = AcpAgentServer::new();
        let req = ZedRequest {
            method: "unknown".to_string(),
            params: serde_json::json!({}),
        };
        let response = tokio::runtime::Runtime::new().unwrap().block_on(server.handle_request(req));
        assert!(matches!(response, AcpResponse::Error { .. }));
    }

    #[test]
    fn handle_request_prompt_orchestrator_unreachable_returns_error() {
        let mut server = AcpAgentServer::new();
        let req = ZedRequest {
            method: "prompt".to_string(),
            params: serde_json::json!({
                "session_id": "abc",
                "message": "hello"
            }),
        };
        let response = tokio::runtime::Runtime::new().unwrap().block_on(server.handle_request(req));
        assert!(matches!(response, AcpResponse::Error { .. }));
    }

    #[test]
    fn handle_request_tool_call_denied_by_gate_returns_error() {
        let mut server = AcpAgentServer::new();
        let req = ZedRequest {
            method: "new_session".to_string(),
            params: serde_json::json!({ "cwd": "/tmp" }),
        };
        let _session = tokio::runtime::Runtime::new().unwrap().block_on(server.handle_request(req));
        let req2 = ZedRequest {
            method: "tool_call".to_string(),
            params: serde_json::json!({
                "session_id": "nonexistent",
                "function_name": "run_command",
                "arguments": "[]"
            }),
        };
        let response = tokio::runtime::Runtime::new().unwrap().block_on(server.handle_request(req2));
        assert!(matches!(response, AcpResponse::Error { .. }));
    }
}
