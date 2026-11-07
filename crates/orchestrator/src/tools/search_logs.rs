use serde::Deserialize;

/// Represents a search_logs request.
#[derive(Debug, Deserialize)]
struct SearchLogsRequest {
    term: String,
    limit: Option<i64>,
}

pub async fn handle(args: &[String], ctx: &super::ToolContext) -> Result<String, String> {
    if args.len() < 2 {
        return Err("Error: search_logs requires at least 1 argument (query)".to_string());
    }

    let search_req: SearchLogsRequest =
        match serde_json::from_str(&serde_json::to_string(args).unwrap_or_default()) {
            Ok(req) => req,
            Err(e) => {
                return Err(format!("Error parsing search_logs arguments: {}", e));
            }
        };

    let store_lock = ctx.store.lock().map_err(|e| format!("Failed to lock store: {}", e))?;
    let events = match store_lock.search_content(&search_req.term, search_req.limit.map(|l| l as usize)) {
        Ok(events) => events,
        Err(e) => {
            return Err(format!("Error searching events: {:?}", e));
        }
    };

    let mut output = String::new();
    output.push_str(&format!(
        "Found {} entries matching '{}':\n",
        events.len(),
        search_req.term
    ));

    for event in events.iter().take(search_req.limit.unwrap_or(10) as usize) {
        output.push_str(&format!(
            "[id:{}] {}\n",
            event.id,
            &event.content[..event.content.len().min(200)]
        ));
    }

    Ok(output)
}
