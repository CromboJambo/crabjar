/// Submit a goal to the conductor API via HTTP POST.
pub async fn submit_goal_to_conductor(
    url: &str,
    body: &serde_json::Value,
    token: &str,
) -> Result<serde_json::Value, String> {
    // Use reqwest for HTTP requests (already a dependency of crabjar-conductor)
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("failed to build HTTP client: {}", e))?;

    let mut request = client.post(url).json(body);

    if !token.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;
    let status = response.status();
    let body_text = response
        .text()
        .await
        .map_err(|e| format!("failed to read response: {}", e))?;

    if !status.is_success() {
        return Err(format!(
            "conductor returned {} error: {}",
            status.as_u16(),
            body_text
        ));
    }

    // Parse response JSON
    serde_json::from_str(&body_text)
        .map_err(|e| format!("failed to parse conductor response: {}", e))
}
