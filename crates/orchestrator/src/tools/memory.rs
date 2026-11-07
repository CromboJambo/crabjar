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
        Ok(response) => {
            serde_json::to_string_pretty(&response)
                .map_err(|e| format!("Error serializing memory response: {}", e))
        }
        Err(e) => Err(format!("Memory tool error: {}", e)),
    }
}
