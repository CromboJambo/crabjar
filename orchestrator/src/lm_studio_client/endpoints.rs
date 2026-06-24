//! Endpoint implementations for native, OpenAI-compatible, and Anthropic-compatible APIs.

use super::types::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::debug;

// ---------------------------------------------------------------------------
// Endpoint implementations
// ---------------------------------------------------------------------------

// Native `/api/v1/chat` endpoint implementation.
pub(super) mod native {
    use super::*;

    /// Converts a unified request to the native endpoint format.
    pub fn to_native_request(req: &UnifiedChatRequest) -> serde_json::Value {
        let mut builder = serde_json::Map::new();

        builder.insert(
            "model".to_string(),
            serde_json::Value::String(req.model.clone()),
        );

        // Convert input to native format.
        let input_obj = serde_json::json!({
            "type": "message",
            "content": req.input.content
        });
        builder.insert(
            "input".to_string(),
            serde_json::Value::Array(vec![input_obj]),
        );

        if let Some(ref system_prompt) = req.system_prompt {
            builder.insert(
                "system_prompt".to_string(),
                serde_json::Value::String(system_prompt.clone()),
            );
        }

        if let Some(temp) = req.temperature {
            builder.insert(
                "temperature".to_string(),
                serde_json::to_value(temp).unwrap_or(serde_json::Value::Null),
            );
        }

        if let Some(top_p) = req.top_p {
            builder.insert(
                "top_p".to_string(),
                serde_json::to_value(top_p).unwrap_or(serde_json::Value::Null),
            );
        }

        if let Some(max_tokens) = req.max_output_tokens {
            builder.insert(
                "max_output_tokens".to_string(),
                serde_json::Value::Number(max_tokens.into()),
            );
        }

        if let Some(ctx_len) = req.context_length {
            builder.insert(
                "context_length".to_string(),
                serde_json::Value::Number(ctx_len.into()),
            );
        }

        if let Some(ref prev_id) = req.previous_response_id {
            builder.insert(
                "previous_response_id".to_string(),
                serde_json::Value::String(prev_id.clone()),
            );
        }

        if let Some(store) = req.store {
            builder.insert("store".to_string(), serde_json::Value::Bool(store));
        }

        if let Some(reasoning) = req.reasoning {
            let reasoning_str = match reasoning {
                ReasoningLevel::Off => "off",
                ReasoningLevel::Low => "low",
                ReasoningLevel::Medium => "medium",
                ReasoningLevel::High => "high",
                ReasoningLevel::On => "on",
            };
            builder.insert(
                "reasoning".to_string(),
                serde_json::Value::String(reasoning_str.to_string()),
            );
        }

        serde_json::Value::Object(builder)
    }

    /// Converts a native response to the unified format.
    pub fn from_native_response(
        value: &serde_json::Value,
    ) -> Result<UnifiedChatResponse, native_error::NativeError> {
        let model_instance_id = value
            .get("model_instance_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let output_items: Vec<UnifiedOutputItem> = value
            .get("output")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(parse_output_item).collect())
            .unwrap_or_default();

        let stats = value
            .get("stats")
            .and_then(|v| serde_json::from_value::<UnifiedStats>(v.clone()).ok());

        let response_id = value
            .get("response_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(UnifiedChatResponse {
            model_instance_id,
            output: output_items,
            stats,
            response_id,
        })
    }

    /// Parses a single output item from a native response.
    fn parse_output_item(item: &serde_json::Value) -> Option<UnifiedOutputItem> {
        let item_type = item.get("type")?.as_str()?;

        match item_type {
            "message" => {
                let content = item.get("content")?.as_str()?.to_string();
                Some(UnifiedOutputItem::Message { content })
            }
            "tool_call" => {
                let tool = item.get("tool")?.as_str()?.to_string();
                let arguments = item.get("arguments")?.clone();
                let output = item.get("output")?.as_str().map(|s| s.to_string());
                let provider_info = item
                    .get("provider_info")
                    .and_then(|v| serde_json::from_value::<ToolProviderInfo>(v.clone()).ok());
                Some(UnifiedOutputItem::ToolCall {
                    tool,
                    arguments,
                    output,
                    provider_info,
                })
            }
            "reasoning" => {
                let content = item.get("content")?.as_str()?.to_string();
                Some(UnifiedOutputItem::Reasoning { content })
            }
            "invalid_tool_call" => {
                let reason = item.get("reason")?.as_str()?.to_string();
                let metadata = item.get("metadata")?.clone();
                let tool_name = item.get("tool_name")?.as_str().map(|s| s.to_string());
                let provider_info = item
                    .get("provider_info")
                    .and_then(|v| serde_json::from_value::<ToolProviderInfo>(v.clone()).ok());
                Some(UnifiedOutputItem::InvalidToolCall {
                    reason,
                    metadata: Some(metadata),
                    tool_name,
                    provider_info,
                })
            }
            _ => None,
        }
    }

    /// Native endpoint error types.
    pub mod native_error {
        use thiserror::Error;

        #[derive(Debug, Error)]
        pub enum NativeError {
            #[error("failed to parse native response: {0}")]
            ParseError(String),
            #[error("request failed: {0}")]
            RequestError(String),
        }
    }
}

// OpenAI-compatible `/v1/chat/completions` endpoint implementation.
pub(super) mod openai {
    use super::*;

    /// Converts a unified request to the OpenAI format.
    pub fn to_openai_request(req: &UnifiedChatRequest) -> serde_json::Value {
        let mut messages: Vec<serde_json::Value> = Vec::new();

        // Add system message if present.
        if let Some(ref system_prompt) = req.system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": system_prompt
            }));
        }

        // Add the input message.
        let role_str = match req.input.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
        };
        messages.push(serde_json::json!({
            "role": role_str,
            "content": req.input.content
        }));

        let mut builder = serde_json::Map::new();
        builder.insert(
            "model".to_string(),
            serde_json::Value::String(req.model.clone()),
        );
        builder.insert("messages".to_string(), serde_json::Value::Array(messages));

        if let Some(temp) = req.temperature {
            builder.insert(
                "temperature".to_string(),
                serde_json::to_value(temp).unwrap_or(serde_json::Value::Null),
            );
        }

        if let Some(max_tokens) = req.max_output_tokens {
            builder.insert(
                "max_output_tokens".to_string(),
                serde_json::Value::Number(max_tokens.into()),
            );
        }

        serde_json::Value::Object(builder)
    }

    /// Converts an OpenAI response to the unified format.
    pub fn from_openai_response(
        value: &serde_json::Value,
    ) -> Result<UnifiedChatResponse, openai_error::OpenaiError> {
        let choices = value.get("choices").and_then(|v| v.as_array()).ok_or(
            openai_error::OpenaiError::ParseError("missing choices in response".to_string()),
        )?;

        if choices.is_empty() {
            return Err(openai_error::OpenaiError::ParseError(
                "empty choices in response".to_string(),
            ));
        }

        let first_choice = &choices[0];
        let message = first_choice
            .get("message")
            .ok_or(openai_error::OpenaiError::ParseError(
                "missing message in choice".to_string(),
            ))?;

        let content = message
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let tool_calls: Vec<UnifiedOutputItem> = message
            .get("tool_calls")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(parse_tool_call).collect())
            .unwrap_or_default();

        // Combine message and tool calls into output items.
        let mut output_items = Vec::new();
        if !content.is_empty() {
            output_items.push(UnifiedOutputItem::Message { content });
        }
        output_items.extend(tool_calls);

        let stats = value
            .get("usage")
            .and_then(|v| serde_json::from_value::<UnifiedStats>(v.clone()).ok());

        Ok(UnifiedChatResponse {
            model_instance_id: value
                .get("model")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            output: output_items,
            stats,
            response_id: None,
        })
    }

    /// Parses a tool call from an OpenAI response.
    fn parse_tool_call(tc: &serde_json::Value) -> Option<UnifiedOutputItem> {
        let function = tc.get("function")?;
        let name = function.get("name")?.as_str()?.to_string();
        let arguments_str = function.get("arguments")?.as_str()?;
        let arguments = serde_json::from_str(arguments_str).ok()?;

        Some(UnifiedOutputItem::ToolCall {
            tool: name,
            arguments,
            output: None,
            provider_info: None,
        })
    }

    /// OpenAI endpoint error types.
    pub mod openai_error {
        use thiserror::Error;

        #[derive(Debug, Error)]
        pub enum OpenaiError {
            #[error("failed to parse OpenAI response: {0}")]
            ParseError(String),
            #[error("request failed: {0}")]
            RequestError(String),
        }
    }
}

// Anthropic-compatible `/v1/messages` endpoint implementation.
pub(super) mod anthropic {
    use super::*;

    /// Converts a unified request to the Anthropic format.
    pub fn to_anthropic_request(req: &UnifiedChatRequest) -> serde_json::Value {
        let mut messages: Vec<serde_json::Value> = Vec::new();

        // Anthropic doesn't have a system message field at the top level.
        // System messages are prepended to the first user message.
        if let Some(ref system_prompt) = req.system_prompt {
            messages.push(serde_json::json!({
                "role": "user",
                "content": format!("[System prompt: {}]", system_prompt)
            }));
        }

        // Add the input message.
        let role_str = match req.input.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
        };
        messages.push(serde_json::json!({
            "role": role_str,
            "content": req.input.content
        }));

        let mut builder = serde_json::Map::new();
        builder.insert(
            "model".to_string(),
            serde_json::Value::String(req.model.clone()),
        );
        builder.insert("messages".to_string(), serde_json::Value::Array(messages));

        if let Some(temp) = req.temperature {
            builder.insert(
                "temperature".to_string(),
                serde_json::to_value(temp).unwrap_or(serde_json::Value::Null),
            );
        }

        if let Some(max_tokens) = req.max_output_tokens {
            builder.insert(
                "max_output_tokens".to_string(),
                serde_json::Value::Number(max_tokens.into()),
            );
        }

        serde_json::Value::Object(builder)
    }

    /// Converts an Anthropic response to the unified format.
    pub fn from_anthropic_response(
        value: &serde_json::Value,
    ) -> Result<UnifiedChatResponse, anthropic_error::AnthropicError> {
        let content_blocks = value.get("content").and_then(|v| v.as_array()).ok_or(
            anthropic_error::AnthropicError::ParseError("missing content in response".to_string()),
        )?;

        let mut output_items = Vec::new();
        for block in content_blocks {
            let block_type = block
                .get("type")
                .ok_or(anthropic_error::AnthropicError::ParseError(
                    "missing type".to_string(),
                ))?
                .as_str()
                .ok_or(anthropic_error::AnthropicError::ParseError(
                    "type not a string".to_string(),
                ))?;

            match block_type {
                "text" => {
                    let content = block
                        .get("text")
                        .ok_or(anthropic_error::AnthropicError::ParseError(
                            "missing text".to_string(),
                        ))?
                        .as_str()
                        .ok_or(anthropic_error::AnthropicError::ParseError(
                            "text not a string".to_string(),
                        ))?
                        .to_string();
                    output_items.push(UnifiedOutputItem::Message { content });
                }
                "tool_use" => {
                    let name = block
                        .get("name")
                        .ok_or(anthropic_error::AnthropicError::ParseError(
                            "missing name".to_string(),
                        ))?
                        .as_str()
                        .ok_or(anthropic_error::AnthropicError::ParseError(
                            "name not a string".to_string(),
                        ))?
                        .to_string();
                    let input = block
                        .get("input")
                        .ok_or(anthropic_error::AnthropicError::ParseError(
                            "missing input".to_string(),
                        ))?
                        .clone();
                    output_items.push(UnifiedOutputItem::ToolCall {
                        tool: name,
                        arguments: input,
                        output: None,
                        provider_info: None,
                    });
                }
                _ => {}
            }
        }

        let stats = value
            .get("usage")
            .and_then(|v| serde_json::from_value::<UnifiedStats>(v.clone()).ok());

        Ok(UnifiedChatResponse {
            model_instance_id: value
                .get("model")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            output: output_items,
            stats,
            response_id: None,
        })
    }

    /// Anthropic endpoint error types.
    pub mod anthropic_error {
        use thiserror::Error;

        #[derive(Debug, Error)]
        pub enum AnthropicError {
            #[error("failed to parse Anthropic response: {0}")]
            ParseError(String),
            #[error("request failed: {0}")]
            RequestError(String),
        }
    }
}
