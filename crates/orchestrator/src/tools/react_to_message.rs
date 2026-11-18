//! react_to_message tool — emoji reactions to messages (P2 parity).
//!
//! In Hermes this reacts to messages in the chat UI. For crabjar (headless),
//! this logs the reaction and returns confirmation. The tool exists for
//! drop-in replacement parity — agents that call it won't error out.

use serde_json::{json, Value};

/// Handle react_to_message tool invocation.
///
/// Arguments:
/// - args[0]: JSON string with fields:
///   - `emoji` (required): The emoji to react with (e.g. "❤️", "😂", "👍")
///   - `message_row_id` (optional): Specific message to react to
///   - `messages_back` (optional): React to an earlier message (1 = one before latest)
pub async fn handle(args: &[String]) -> Result<String, String> {
    if args.is_empty() || args[0].is_empty() {
        return Err("react_to_message requires a JSON argument with 'emoji' field".to_string());
    }

    let request: Value = serde_json::from_str(&args[0])
        .map_err(|e| format!("Failed to parse react_to_message arguments: {}", e))?;

    let emoji = request["emoji"]
        .as_str()
        .ok_or("react_to_message requires 'emoji' field")?
        .to_string();

    if emoji.is_empty() {
        return Err("emoji cannot be empty (pass empty string to remove reaction)".to_string());
    }

    let message_row_id = request["message_row_id"].as_u64();
    let messages_back = request["messages_back"].as_u64();

    // In headless mode, we just acknowledge the reaction.
    // If a desktop UI is added later, this would send the reaction to the UI layer.
    let result = json!({
        "status": "ok",
        "action": "reacted",
        "emoji": emoji,
        "message_row_id": message_row_id,
        "messages_back": messages_back,
        "note": "Reaction acknowledged (headless mode — no UI to display)"
    });

    Ok(serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_react_to_message_basic() {
        let args = vec![json!({"emoji": "👍"}).to_string()];
        let result = handle(&args).await.unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["status"], "ok");
        assert_eq!(parsed["emoji"], "👍");
    }

    #[tokio::test]
    async fn test_react_to_message_with_row_id() {
        let args = vec![json!({"emoji": "❤️", "message_row_id": 42}).to_string()];
        let result = handle(&args).await.unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["message_row_id"], 42);
    }

    #[tokio::test]
    async fn test_react_to_message_with_messages_back() {
        let args = vec![json!({"emoji": "😂", "messages_back": 1}).to_string()];
        let result = handle(&args).await.unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["messages_back"], 1);
    }

    #[tokio::test]
    async fn test_react_to_message_missing_emoji() {
        let args = vec![json!({}).to_string()];
        let result = handle(&args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("requires 'emoji'"));
    }

    #[tokio::test]
    async fn test_react_to_message_empty_args() {
        let args: Vec<String> = vec![];
        let result = handle(&args).await;
        assert!(result.is_err());
    }
}
