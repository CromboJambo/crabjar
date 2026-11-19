pub async fn handle(args: &[String], _ctx: &super::ToolContext) -> Result<String, String> {
    if args.is_empty() {
        return Err("Error: memory requires JSON arguments".to_string());
    }

    let input_json = &args[0];
    let input: crabjar_memory::MemoryToolInput = serde_json::from_str(input_json)
        .map_err(|e| format!("Error parsing memory tool arguments: {}", e))?;

    // Determine memory directory (use ~/.hermes/memories/ for drop-in compatibility)
    let home_dir = dirs::home_dir().ok_or("Error: could not determine home directory")?;
    let memories_dir = home_dir.join(".hermes").join("memories");

    // Create the tool and execute
    let tool = crabjar_memory::MemoryTool::new(&memories_dir);
    match tool.execute(input) {
        Ok(response) => serde_json::to_string_pretty(&response)
            .map_err(|e| format!("Error serializing memory response: {}", e)),
        Err(e) => Err(format!("Memory tool error: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crabjar_memory::{MemoryTool, MemoryToolInput};
    use tempfile::tempdir;

    #[test]
    fn test_memory_add_single() {
        let dir = tempdir().unwrap();
        let tool = MemoryTool::new(dir.path());
        let input: MemoryToolInput = serde_json::from_str(
            r#"{"target": "memory", "action": "add", "content": "Test fact"}"#,
        )
        .unwrap();
        let resp = tool.execute(input).unwrap();
        assert!(resp.success);
        assert_eq!(resp.entries_count, 1);
    }

    #[test]
    fn test_memory_batch_operations() {
        let dir = tempdir().unwrap();
        let tool = MemoryTool::new(dir.path());
        let input: MemoryToolInput = serde_json::from_str(
            r#"{"target": "memory", "operations": [
                {"action": "add", "content": "Entry one"},
                {"action": "add", "content": "Entry two"}
            ]}"#,
        )
        .unwrap();
        let resp = tool.execute(input).unwrap();
        assert!(resp.success);
        assert_eq!(resp.entries_count, 2);
    }

    #[test]
    fn test_memory_patch() {
        let dir = tempdir().unwrap();
        let tool = MemoryTool::new(dir.path());
        // Add first
        let add_input: MemoryToolInput = serde_json::from_str(
            r#"{"target": "memory", "action": "add", "content": "Old text here"}"#,
        )
        .unwrap();
        tool.execute(add_input).unwrap();
        // Patch
        let patch_input: MemoryToolInput = serde_json::from_str(
            r#"{"target": "memory", "action": "patch", "old_text": "Old", "content": "New"}"#,
        )
        .unwrap();
        let resp = tool.execute(patch_input).unwrap();
        assert!(resp.success);
        assert_eq!(resp.entries_count, 1);
    }

    #[test]
    fn test_memory_remove() {
        let dir = tempdir().unwrap();
        let tool = MemoryTool::new(dir.path());
        // Add first
        let add_input: MemoryToolInput = serde_json::from_str(
            r#"{"target": "memory", "action": "add", "content": "To be removed"}"#,
        )
        .unwrap();
        tool.execute(add_input).unwrap();
        // Remove
        let remove_input: MemoryToolInput = serde_json::from_str(
            r#"{"target": "memory", "action": "remove", "old_text": "removed"}"#,
        )
        .unwrap();
        let resp = tool.execute(remove_input).unwrap();
        assert!(resp.success);
        assert_eq!(resp.entries_count, 0);
    }

    #[test]
    fn test_memory_user_target() {
        let dir = tempdir().unwrap();
        let tool = MemoryTool::new(dir.path());
        let input: MemoryToolInput = serde_json::from_str(
            r#"{"target": "user", "action": "add", "content": "User preference"}"#,
        )
        .unwrap();
        let resp = tool.execute(input).unwrap();
        assert!(resp.success);
    }

    #[test]
    fn test_memory_invalid_target() {
        let dir = tempdir().unwrap();
        let tool = MemoryTool::new(dir.path());
        let input: MemoryToolInput =
            serde_json::from_str(r#"{"target": "invalid", "action": "add", "content": "Test"}"#)
                .unwrap();
        let result = tool.execute(input);
        assert!(result.is_err());
    }
}
