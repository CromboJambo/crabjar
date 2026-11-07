pub async fn handle(args: &[String], _ctx: &super::ToolContext) -> Result<String, String> {
    if args.is_empty() {
        return Err("Error: browser_exec requires JSON arguments".to_string());
    }

    let input_json = &args[0];
    #[derive(serde::Deserialize)]
    struct BrowserExecInput {
        code: String,
    }
    let input: BrowserExecInput = serde_json::from_str(input_json)
        .map_err(|e| format!("Error parsing browser_exec arguments: {}", e))?;

    match crabjar_browser::execute_code(&input.code).await {
        Ok(result) => Ok(serde_json::to_string_pretty(&result)
            .map_err(|e| format!("Failed to serialize result: {}", e))?),
        Err(e) => Err(format!("Browser code execution failed: {}", e)),
    }
}
