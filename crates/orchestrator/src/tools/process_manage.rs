pub async fn handle(args: &[String], ctx: &super::ToolContext) -> Result<String, String> {
    if args.is_empty() {
        return Err("Error: process_manage requires JSON arguments".to_string());
    }

    let input_json = &args[0];
    let input: crate::process_management::ProcessManageRequest = serde_json::from_str(input_json)
        .map_err(|e| format!("Error parsing process_manage arguments: {}", e))?;

    let response = crate::process_management::execute_process_management(input, &ctx.process_registry).await;
    match response {
        Ok(result) => Ok(serde_json::to_string(&result)
            .map_err(|e| format!("Failed to serialize process management result: {}", e))?),
        Err(e) => Err(format!("Process management failed: {}", e)),
    }
}
