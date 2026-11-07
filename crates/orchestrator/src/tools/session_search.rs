pub async fn handle(args: &[String], _ctx: &super::ToolContext) -> Result<String, String> {
    if args.is_empty() {
        return Err("Error: session_search requires JSON arguments".to_string());
    }

    let input_json = &args[0];
    let input: crate::session_search::SessionSearchRequest = serde_json::from_str(input_json)
        .map_err(|e| format!("Error parsing session_search arguments: {}", e))?;

    let response = crate::session_search::execute_session_search(input).await;
    match response {
        Ok(result) => Ok(serde_json::to_string(&result)
            .map_err(|e| format!("Failed to serialize session search result: {}", e))?),
        Err(e) => Err(format!("Session search failed: {}", e)),
    }
}
