use serde::Deserialize;

/// Represents a by_source request.
#[derive(Debug, Deserialize)]
struct BySourceRequest {
    source: String,
    limit: Option<i64>,
}

pub async fn handle(args: &[String], ctx: &super::ToolContext) -> Result<String, String> {
    let source_req: BySourceRequest =
        match serde_json::from_str(&serde_json::to_string(args).unwrap_or_default()) {
            Ok(req) => req,
            Err(e) => {
                return Err(format!("Error parsing by_source arguments: {}", e));
            }
        };

    let store_lock = ctx.store.lock().map_err(|e| format!("Failed to lock store: {}", e))?;
    let events = match store_lock.query(
        &[&source_req.source],
        source_req.limit.unwrap_or(50) as usize,
        "",
        "",
        "",
    ) {
        Ok(events) => events,
        Err(e) => {
            return Err(format!("Error fetching events by source: {:?}", e));
        }
    };

    let mut output = String::new();
    output.push_str(&format!("Events from source '{}':\n", source_req.source));

    for event in events.iter() {
        output.push_str(&format!(
            "[id:{}] {}\n",
            event.id,
            &event.content[..event.content.len().min(200)]
        ));
    }

    Ok(output)
}
