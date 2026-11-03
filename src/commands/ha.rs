use crabjar_lib::HaCommand;
use serde_json::json;

fn error_response(message: &str) -> serde_json::Value {
    json!({
        "success": false,
        "error": message,
    })
}

/// Handle Home Assistant commands (P2-2 Hermes compatibility layer)
pub fn handle_ha_command(command: HaCommand) -> serde_json::Value {
    // Placeholder implementation — actual Home Assistant API integration would go here
    match command {
        HaCommand::GetState { entity_id } => json!({
            "success": true,
            "message": format!("retrieved state for {}", entity_id),
            "data": {
                "entity_id": entity_id.clone(),
                "state": "unknown",
                "attributes": {}
            }
        }),
        HaCommand::ListEntities { domain } => json!({
            "success": true,
            "message": format!("retrieved entities for domain: {:?}", domain),
            "data": [],
            "count": 0
        }),
        HaCommand::CallService { domain, service, entity_id, data: _ } => json!({
            "success": true,
            "message": format!("called {}.{} on {}", domain, service, entity_id.as_deref().unwrap_or_default()),
            "data": {
                "domain": domain,
                "service": service,
                "entity_id": entity_id
            }
        }),
        HaCommand::ListServices => json!({
            "success": true,
            "message": "retrieved available services",
            "data": []
        }),
    }
}
