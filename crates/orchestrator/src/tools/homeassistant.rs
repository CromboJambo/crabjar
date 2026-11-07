pub async fn handle(function_name: &str, args: &[String], _ctx: &super::ToolContext) -> Result<String, String> {
    if args.is_empty() {
        return Err(format!("Error: {} requires JSON arguments", function_name));
    }

    let input_json = &args[0];
    let input: serde_json::Value = serde_json::from_str(input_json)
        .map_err(|e| format!("Error parsing {} arguments: {}", function_name, e))?;

    let result = match function_name {
        "ha_get_state" => crabjar_homeassistant::tool_ha_get_state(&input).await,
        "ha_call_service" => crabjar_homeassistant::tool_ha_call_service(&input).await,
        "ha_list_entities" => crabjar_homeassistant::tool_ha_list_entities(&input).await,
        "ha_list_services" => crabjar_homeassistant::tool_ha_list_services(&input).await,
        _ => unreachable!(),
    };

    match result {
        Ok(value) => Ok(serde_json::to_string_pretty(&value)
            .map_err(|e| format!("Failed to serialize HA result: {}", e))?),
        Err(e) => Err(format!("Home Assistant tool failed: {}", e)),
    }
}
