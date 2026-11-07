use serde::Deserialize;

/// Represents a recent_events request.
#[derive(Debug, Deserialize)]
struct RecentEventsRequest {
    limit: i64,
}

pub async fn handle(args: &[String], ctx: &super::ToolContext) -> Result<String, String> {
    let recent_req: RecentEventsRequest =
        match serde_json::from_str(&serde_json::to_string(args).unwrap_or_default()) {
            Ok(req) => req,
            Err(e) => {
                return Err(format!("Error parsing recent_events arguments: {}", e));
            }
        };

    let store_lock = ctx.store.lock().map_err(|e| format!("Failed to lock store: {}", e))?;
    let events = match store_lock.events(recent_req.limit as usize) {
        Ok(events) => events,
        Err(e) => {
            return Err(format!("Error fetching recent events: {:?}", e));
        }
    };

    let mut output = String::new();
    output.push_str(&format!("Recent {} events:\n", events.len()));

    for event in events.iter() {
        output.push_str(&format!(
            "[{}] {} - {}\n",
            event.timestamp.format("%Y-%m-%d %H:%M:%S"),
            event.event_type,
            event.id,
        ));
    }

    Ok(output)
}
