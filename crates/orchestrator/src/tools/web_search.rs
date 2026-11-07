pub async fn handle(args: &[String]) -> Result<String, String> {
    if args.len() < 2 {
        return Err("Error: web_search requires at least 2 arguments (tool and query)".to_string());
    }

    let query = &args[0];
    let limit_str = &args[1];
    let limit: Option<usize> = limit_str.parse().ok();

    match crabjar_web_search::search(query, limit).await {
        Ok(response) => {
            if response.results.is_empty() {
                return Ok("No search results found.".to_string());
            }

            let mut output = String::new();
            output.push_str(&format!("Found {} search results:\n\n", response.results.len()));

            for (i, result) in response.results.iter().enumerate() {
                output.push_str(&format!("{}. {}\n", i + 1, result.title));
                output.push_str(&format!("   URL: {}\n", result.url));
                if let Some(desc) = &result.description {
                    if !desc.is_empty() {
                        output.push_str(&format!("   Description: {}\n", desc));
                    }
                }
                if let Some(snippet) = &result.snippet {
                    if !snippet.is_empty() {
                        output.push_str(&format!("   Snippet: {}\n", snippet));
                    }
                }
                output.push('\n');
            }

            Ok(output)
        }
        Err(e) => Err(format!("Web search failed: {}", e)),
    }
}
