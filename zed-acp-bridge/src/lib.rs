#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_schema_from_function_call_run_command_works() {
        let schema = ToolSchema::from_function_call("run_command", r#"["echo", "hello"]"#).unwrap();
        assert_eq!(schema.tool, "echo");
        assert_eq!(schema.args, vec!["hello".to_string()]);
        assert!(schema.requires_guard);
    }

    #[test]
    fn tool_schema_from_function_call_search_logs_works() {
        let schema = ToolSchema::from_function_call("search_logs", r#"[]"#).unwrap();
        assert_eq!(schema.tool, "crabjar");
        assert_eq!(schema.args, vec!["guard".to_string(), "queue".to_string()]);
        assert!(!schema.requires_guard);
    }

    #[test]
    fn tool_schema_from_function_call_recent_events_works() {
        let schema = ToolSchema::from_function_call("recent_events", r#"[]"#).unwrap();
        assert_eq!(schema.tool, "crabjar");
        assert_eq!(schema.args, vec!["knowledge".to_string(), "events".to_string()]);
        assert!(!schema.requires_guard);
    }

    #[test]
    fn tool_schema_from_function_call_by_source_works() {
        let schema = ToolSchema::from_function_call("by_source", r#"[]"#).unwrap();
        assert_eq!(schema.tool, "crabjar");
        assert_eq!(schema.args, vec!["knowledge".to_string(), "events".to_string()]);
        assert!(!schema.requires_guard);
    }

    #[test]
    fn tool_schema_from_function_call_analyze_state_works() {
        let schema = ToolSchema::from_function_call("analyze_state", r#"[]"#).unwrap();
        assert_eq!(schema.tool, "crabjar");
        assert_eq!(schema.args, vec!["state".to_string(), "list".to_string()]);
        assert!(!schema.requires_guard);
    }

    #[test]
    fn tool_schema_from_function_call_unknown_returns_passthrough() {
        let schema = ToolSchema::from_function_call("unknown_tool", r#"["arg1", "arg2"]"#).unwrap();
        assert_eq!(schema.tool, "unknown_tool");
        assert_eq!(schema.args, vec!["arg1".to_string(), "arg2".to_string()]);
        assert!(!schema.requires_guard);
    }

    #[test]
    fn tool_schema_from_function_call_parse_error_errors() {
        let result = ToolSchema::from_function_call("run_command", "not valid json");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("parse error"));
    }

    #[test]
    fn tool_schema_from_function_call_empty_args_run_command() {
        let schema = ToolSchema::from_function_call("run_command", r#"[]"#).unwrap();
        assert_eq!(schema.tool, "");
        assert!(schema.args.is_empty());
        assert!(schema.requires_guard);
    }

    #[test]
    fn acp_session_new_with_custom_cwd() {
        let session = AcpSession::new("/custom/path".to_string());
        assert_eq!(session.cwd, "/custom/path");
    }

    #[test]
    fn acp_bridge_new_session_creates_session() {
        let bridge = AcpBridge::new();
        let session = bridge.new_session("/tmp/test".to_string()).unwrap();
        assert!(!session.id.is_empty());
        assert_eq!(session.cwd, "/tmp/test");
    }

    #[test]
    fn acp_bridge_load_session_works() {
        let mut bridge = AcpBridge::new();
        let session = bridge.new_session("/tmp/test".to_string()).unwrap();
        let loaded = bridge.load_session(session.id.clone(), "/tmp/new".to_string()).unwrap();
        assert_eq!(loaded.cwd, "/tmp/new");
    }

    #[test]
    fn acp_bridge_load_session_not_found_errors() {
        let mut bridge = AcpBridge::new();
        let result = bridge.load_session("nonexistent".to_string(), "/tmp".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn acp_bridge_close_session_works() {
        let mut bridge = AcpBridge::new();
        let session = bridge.new_session("/tmp/test".to_string()).unwrap();
        bridge.close_session(session.id.clone()).unwrap();
        assert!(bridge.list_sessions().is_empty());
    }

    #[test]
    fn acp_bridge_close_session_nonexistent_noop() {
        let mut bridge = AcpBridge::new();
        bridge.close_session("nonexistent".to_string()).unwrap();
        assert!(bridge.list_sessions().is_empty());
    }

    #[test]
    fn acp_bridge_list_sessions_returns_all() {
        let mut bridge = AcpBridge::new();
        bridge.new_session("/tmp/a".to_string()).unwrap();
        bridge.new_session("/tmp/b".to_string()).unwrap();
        let sessions = bridge.list_sessions();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn acp_bridge_record_event_works() {
        let mut bridge = AcpBridge::new();
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
        let mut bridge = AcpBridge::new();
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
    fn map_tool_call_passthrough_for_unknown() {
        let bridge = AcpBridge::new();
        let schema = bridge.map_tool_call("unknown", r#"[]"#).unwrap();
        assert_eq!(schema.tool, "unknown");
        assert!(!schema.requires_guard);
    }
}
