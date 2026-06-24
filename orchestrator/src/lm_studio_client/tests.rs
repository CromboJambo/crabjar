//! Tests for lm_studio_client submodules.

use super::*;
use serial_test::serial;

#[test]
fn endpoint_native_path() {
    let ep = LmStudioEndpoint::Native;
    assert_eq!(ep.path(), "/api/v1/chat");
}

#[test]
fn endpoint_openai_path() {
    let ep = LmStudioEndpoint::Openai;
    assert_eq!(ep.path(), "/v1/chat/completions");
}

#[test]
fn endpoint_anthropic_path() {
    let ep = LmStudioEndpoint::Anthropic;
    assert_eq!(ep.path(), "/v1/messages");
}

#[test]
fn endpoint_native_name() {
    let ep = LmStudioEndpoint::Native;
    assert_eq!(ep.name(), "native");
}

#[test]
fn endpoint_openai_name() {
    let ep = LmStudioEndpoint::Openai;
    assert_eq!(ep.name(), "openai-compat");
}

#[test]
fn endpoint_anthropic_name() {
    let ep = LmStudioEndpoint::Anthropic;
    assert_eq!(ep.name(), "anthropic-compat");
}

#[test]
#[serial]
fn endpoint_from_env_default() {
    unsafe {
        std::env::remove_var("LM_STUDIO_ENDPOINT");
    }
    let ep = LmStudioEndpoint::from_env();
    assert_eq!(ep, LmStudioEndpoint::Openai);
}

#[test]
#[serial]
fn endpoint_from_env_native() {
    unsafe {
        std::env::set_var("LM_STUDIO_ENDPOINT", "native");
    }
    let ep = LmStudioEndpoint::from_env();
    assert_eq!(ep, LmStudioEndpoint::Native);
    unsafe {
        std::env::remove_var("LM_STUDIO_ENDPOINT");
    }
}

#[test]
#[serial]
fn endpoint_from_env_openai() {
    unsafe {
        std::env::set_var("LM_STUDIO_ENDPOINT", "openai");
    }
    let ep = LmStudioEndpoint::from_env();
    assert_eq!(ep, LmStudioEndpoint::Openai);
    unsafe {
        std::env::remove_var("LM_STUDIO_ENDPOINT");
    }
}

#[test]
#[serial]
fn endpoint_from_env_anthropic() {
    unsafe {
        std::env::set_var("LM_STUDIO_ENDPOINT", "anthropic");
    }
    let ep = LmStudioEndpoint::from_env();
    assert_eq!(ep, LmStudioEndpoint::Anthropic);
    unsafe {
        std::env::remove_var("LM_STUDIO_ENDPOINT");
    }
}

#[test]
#[serial]
fn endpoint_from_env_invalid_defaults_to_openai() {
    unsafe {
        std::env::set_var("LM_STUDIO_ENDPOINT", "invalid");
    }
    let ep = LmStudioEndpoint::from_env();
    assert_eq!(ep, LmStudioEndpoint::Openai);
    unsafe {
        std::env::remove_var("LM_STUDIO_ENDPOINT");
    }
}

#[test]
fn config_from_env_defaults() {
    unsafe {
        std::env::remove_var("LM_STUDIO_URL");
        std::env::remove_var("LM_STUDIO_MODEL");
        std::env::remove_var("LM_STUDIO_CONTEXT_LENGTH");
        std::env::remove_var("LM_STUDIO_TEMPERATURE");
        std::env::remove_var("LM_STUDIO_MAX_OUTPUT_TOKENS");
        std::env::remove_var("LM_API_TOKEN");
    }

    let config = LmStudioConfig::from_env();
    assert_eq!(config.base_url, "http://127.0.0.1:1234");
    assert_eq!(config.default_model, "local-model");
    assert!(config.api_token.is_none());
    assert!(config.default_context_length.is_none());
    assert!(config.default_temperature.is_none());
    assert!(config.default_max_output_tokens.is_none());
}

#[test]
fn config_endpoint_url_constructed() {
    let config = LmStudioConfig {
        base_url: "http://localhost:1234".to_string(),
        serve_base_url: None,
        endpoint: LmStudioEndpoint::Openai,
        api_token: None,
        default_model: "test-model".to_string(),
        default_context_length: None,
        default_temperature: None,
        default_max_output_tokens: None,
    };
    assert_eq!(
        config.endpoint_url(),
        "http://localhost:1234/v1/chat/completions"
    );
}

#[test]
fn config_endpoint_url_native() {
    let config = LmStudioConfig {
        base_url: "http://example.com:8080".to_string(),
        serve_base_url: None,
        endpoint: LmStudioEndpoint::Native,
        api_token: None,
        default_model: "test".to_string(),
        default_context_length: None,
        default_temperature: None,
        default_max_output_tokens: None,
    };
    assert_eq!(config.endpoint_url(), "http://example.com:8080/api/v1/chat");
}

#[test]
fn config_endpoint_url_anthropic() {
    let config = LmStudioConfig {
        base_url: "http://example.com:8080".to_string(),
        serve_base_url: None,
        endpoint: LmStudioEndpoint::Anthropic,
        api_token: None,
        default_model: "test".to_string(),
        default_context_length: None,
        default_temperature: None,
        default_max_output_tokens: None,
    };
    assert_eq!(config.endpoint_url(), "http://example.com:8080/v1/messages");
}

#[test]
fn message_role_serde_user() {
    let role = MessageRole::User;
    let json = serde_json::to_string(&role).unwrap();
    assert_eq!(json, "\"user\"");
}

#[test]
fn message_role_serde_system() {
    let role = MessageRole::System;
    let json = serde_json::to_string(&role).unwrap();
    assert_eq!(json, "\"system\"");
}

#[test]
fn message_role_serde_assistant() {
    let role = MessageRole::Assistant;
    let json = serde_json::to_string(&role).unwrap();
    assert_eq!(json, "\"assistant\"");
}

#[test]
fn message_role_serde_roundtrip_user() {
    let role = MessageRole::User;
    let json = serde_json::to_string(&role).unwrap();
    let restored: MessageRole = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, role);
}

#[test]
fn unified_message_clone_works() {
    let msg = UnifiedMessage {
        role: MessageRole::User,
        content: "hello".to_string(),
    };
    let cloned = msg.clone();
    assert_eq!(cloned.role, msg.role);
    assert_eq!(cloned.content, msg.content);
}

#[test]
fn unified_chat_request_from_config() {
    let config = LmStudioConfig {
        base_url: "http://localhost:1234".to_string(),
        serve_base_url: None,
        endpoint: LmStudioEndpoint::Openai,
        api_token: Some("token".to_string()),
        default_model: "test-model".to_string(),
        default_context_length: Some(4096),
        default_temperature: Some(0.7),
        default_max_output_tokens: Some(1024),
    };
    let req = UnifiedChatRequest::from_config(&config, "hello".to_string(), None);
    assert_eq!(req.model, "test-model");
    assert_eq!(req.input.content, "hello");
    assert_eq!(req.temperature, Some(0.7));
    assert_eq!(req.max_output_tokens, Some(1024));
    assert_eq!(req.context_length, Some(4096));
}

#[test]
fn unified_chat_request_from_config_with_prev_id() {
    let config = LmStudioConfig {
        base_url: "http://localhost:1234".to_string(),
        serve_base_url: None,
        endpoint: LmStudioEndpoint::Native,
        api_token: None,
        default_model: "test-model".to_string(),
        default_context_length: None,
        default_temperature: None,
        default_max_output_tokens: None,
    };
    let prev_id = Some("resp-123".to_string());
    let req = UnifiedChatRequest::from_config(&config, "world".to_string(), prev_id.clone());
    assert_eq!(req.previous_response_id, prev_id);
}

#[test]
fn session_state_new() {
    let state = SessionState::new();
    assert!(state.response_id.is_none());
    assert!(state.message_history.is_empty());
}

#[test]
fn session_state_with_system_prompt() {
    let state = SessionState::with_system_prompt("You are helpful".to_string());
    assert!(state.response_id.is_none());
    assert_eq!(state.message_history.len(), 1);
    assert_eq!(state.message_history[0].role, MessageRole::System);
    assert_eq!(state.message_history[0].content, "You are helpful");
}

#[test]
fn session_state_update_with_response() {
    let mut state = SessionState::new();
    let response = UnifiedChatResponse {
        model_instance_id: "model-1".to_string(),
        output: vec![UnifiedOutputItem::Message {
            content: "Hello!".to_string(),
        }],
        stats: None,
        response_id: Some("resp-456".to_string()),
    };
    state.update_with_response(&response);
    assert_eq!(state.response_id, Some("resp-456".to_string()));
    assert_eq!(state.message_history.len(), 1);
    assert_eq!(state.message_history[0].role, MessageRole::Assistant);
    assert_eq!(state.message_history[0].content, "Hello!");
}

#[test]
fn session_state_add_user_message() {
    let mut state = SessionState::new();
    state.add_user_message("How are you?".to_string());
    assert_eq!(state.message_history.len(), 1);
    assert_eq!(state.message_history[0].role, MessageRole::User);
    assert_eq!(state.message_history[0].content, "How are you?");
}

#[test]
fn session_state_has_response_id() {
    let mut state = SessionState::new();
    assert!(!state.has_response_id());
    state.response_id = Some("resp-789".to_string());
    assert!(state.has_response_id());
}

#[test]
fn session_store_new() {
    let store = SessionStore::new("/tmp/test-sessions.db".to_string());
    assert_eq!(store.db_path, "/tmp/test-sessions.db");
}

#[test]
fn session_store_create_and_get() {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let store = SessionStore::new(format!("/tmp/test-sessions-get-{}.db", ts));
    let session_id = store.create_session(Some("Test system".to_string())).unwrap();
    let state = store.get_session(&session_id).unwrap();
    assert_eq!(state.response_id, None);
    assert!(state.message_history.is_empty());
}

#[test]
fn session_store_delete() {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let store = SessionStore::new(format!("/tmp/test-sessions-del-{}.db", ts));
    let session_id = store.create_session(None).unwrap();
    store.delete_session(&session_id).unwrap();
    // Delete is idempotent
    assert!(store.delete_session(&session_id).is_ok());
}

#[test]
fn unified_stats_default() {
    let stats = UnifiedStats {
        input_tokens: 10,
        total_output_tokens: 20,
        reasoning_output_tokens: None,
        tokens_per_second: Some(5.0),
        time_to_first_token_seconds: Some(0.1),
        model_load_time_seconds: Some(2.0),
    };
    assert_eq!(stats.input_tokens, 10);
    assert_eq!(stats.total_output_tokens, 20);
    assert_eq!(stats.tokens_per_second, Some(5.0));
}

#[test]
fn tool_provider_info_new() {
    let info = ToolProviderInfo {
        provider_type: "mcp".to_string(),
        plugin_id: Some("plugin-1".to_string()),
        server_label: Some("label".to_string()),
    };
    assert_eq!(info.provider_type, "mcp");
    assert_eq!(info.plugin_id, Some("plugin-1".to_string()));
    assert_eq!(info.server_label, Some("label".to_string()));
}

#[test]
fn tool_call_info_new() {
    let info = ToolCallInfo {
        tool: "get_weather".to_string(),
        arguments: serde_json::json!({"city": "SF"}),
        output: Some("sunny".to_string()),
        provider_info: Some(ToolProviderInfo {
            provider_type: "mcp".to_string(),
            plugin_id: None,
            server_label: None,
        }),
    };
    assert_eq!(info.tool, "get_weather");
    assert_eq!(info.output, Some("sunny".to_string()));
}

#[test]
fn unified_output_item_message() {
    let item = UnifiedOutputItem::Message {
        content: "Hello".to_string(),
    };
    assert!(matches!(item, UnifiedOutputItem::Message { .. }));
}

#[test]
fn unified_output_item_tool_call() {
    let item = UnifiedOutputItem::ToolCall {
        tool: "cmd".to_string(),
        arguments: serde_json::json!({"arg": "val"}),
        output: None,
        provider_info: None,
    };
    assert!(matches!(item, UnifiedOutputItem::ToolCall { .. }));
}

#[test]
fn unified_output_item_reasoning() {
    let item = UnifiedOutputItem::Reasoning {
        content: "Let me think...".to_string(),
    };
    assert!(matches!(item, UnifiedOutputItem::Reasoning { .. }));
}

#[test]
fn unified_output_item_invalid_tool_call() {
    let item = UnifiedOutputItem::InvalidToolCall {
        reason: "missing arg".to_string(),
        metadata: Some(serde_json::json!({"expected": "arg"})),
        tool_name: Some("cmd".to_string()),
        provider_info: None,
    };
    assert!(matches!(item, UnifiedOutputItem::InvalidToolCall { .. }));
}

#[test]
fn reasoning_level_serde() {
    let level = ReasoningLevel::High;
    let json = serde_json::to_string(&level).unwrap();
    assert_eq!(json, "\"high\"");
    let restored: ReasoningLevel = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, ReasoningLevel::High);
}

#[test]
fn endpoint_mistralrs_path() {
    let ep = LmStudioEndpoint::MistralRsServe;
    assert_eq!(ep.path(), "/v1/chat/completions");
    assert_eq!(ep.name(), "mistralrs-serve");
}

#[test]
fn endpoint_mistralrs_from_env() {
    unsafe {
        std::env::set_var("LM_STUDIO_ENDPOINT", "mistralrs");
    }
    let ep = LmStudioEndpoint::from_env();
    assert_eq!(ep, LmStudioEndpoint::MistralRsServe);
    unsafe {
        std::env::remove_var("LM_STUDIO_ENDPOINT");
    }
}

#[test]
fn config_endpoint_url_mistralrs_default() {
    let config = LmStudioConfig {
        base_url: "http://localhost:1234".to_string(),
        serve_base_url: None,
        endpoint: LmStudioEndpoint::MistralRsServe,
        api_token: None,
        default_model: "test".to_string(),
        default_context_length: None,
        default_temperature: None,
        default_max_output_tokens: None,
    };
    // Should use base_url when serve_base_url is None
    assert_eq!(
        config.endpoint_url(),
        "http://localhost:1234/v1/chat/completions"
    );
}

#[test]
fn config_endpoint_url_mistralrs_with_serve() {
    let config = LmStudioConfig {
        base_url: "http://localhost:1234".to_string(),
        serve_base_url: Some("http://localhost:8081".to_string()),
        endpoint: LmStudioEndpoint::MistralRsServe,
        api_token: None,
        default_model: "test".to_string(),
        default_context_length: None,
        default_temperature: None,
        default_max_output_tokens: None,
    };
    assert_eq!(
        config.endpoint_url(),
        "http://localhost:8081/v1/chat/completions"
    );
}

#[test]
fn config_from_env_with_all_vars() {
    unsafe {
        std::env::set_var("LM_STUDIO_URL", "http://custom:9999");
        std::env::set_var("LM_STUDIO_MODEL", "my-model");
        std::env::set_var("LM_STUDIO_CONTEXT_LENGTH", "8192");
        std::env::set_var("LM_STUDIO_TEMPERATURE", "0.9");
        std::env::set_var("LM_STUDIO_MAX_OUTPUT_TOKENS", "2048");
        std::env::set_var("LM_API_TOKEN", "secret");
        std::env::set_var("MISTRALRS_SERVE_URL", "http://serve:8081");
    }

    let config = LmStudioConfig::from_env();
    assert_eq!(config.base_url, "http://custom:9999");
    assert_eq!(config.default_model, "my-model");
    assert_eq!(config.default_context_length, Some(8192));
    assert_eq!(config.default_temperature, Some(0.9));
    assert_eq!(config.default_max_output_tokens, Some(2048));
    assert_eq!(config.api_token, Some("secret".to_string()));
    assert_eq!(config.serve_base_url, Some("http://serve:8081".to_string()));

    unsafe {
        std::env::remove_var("LM_STUDIO_URL");
        std::env::remove_var("LM_STUDIO_MODEL");
        std::env::remove_var("LM_STUDIO_CONTEXT_LENGTH");
        std::env::remove_var("LM_STUDIO_TEMPERATURE");
        std::env::remove_var("LM_STUDIO_MAX_OUTPUT_TOKENS");
        std::env::remove_var("LM_API_TOKEN");
        std::env::remove_var("MISTRALRS_SERVE_URL");
    }
}
