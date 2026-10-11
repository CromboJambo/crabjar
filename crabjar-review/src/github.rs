use reqwest::Client;
use serde_json::{json, Value};

/// GitHub API client for posting review comments and summaries.
pub struct GithubClient {
    client: Client,
    owner: String,
    repo: String,
    pr_number: u32,
    token: String,
}

impl GithubClient {
    pub fn new(owner: &str, repo: &str, pr_number: u32) -> Result<Self, String> {
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| "GITHUB_TOKEN not set")?;

        Ok(Self {
            client: Client::new(),
            owner: owner.to_string(),
            repo: repo.to_string(),
            pr_number,
            token,
        })
    }

    fn api_url(&self, path: &str) -> String {
        format!("https://api.github.com/repos/{}/{}{}", self.owner, self.repo, path)
    }

    /// Post a single review comment on a specific line.
    pub async fn post_review_comment(
        &self,
        file: &str,
        line: u32,
        body: &str,
        severity: &str,
    ) -> Result<Value, String> {
        let url = self.api_url(&format!("/pulls/{}/comments", self.pr_number));

        let payload = json!({
            "path": file,
            "line": line,
            "body": format!("**[{}]** {}", severity.to_uppercase(), body),
        });

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let status = response.status();
        let body = response.json::<Value>().await.map_err(|e| format!("JSON parse failed: {}", e))?;

        if !status.is_success() {
            return Err(format!("GitHub API error {}: {:?}", status, body));
        }

        Ok(body)
    }

    /// Post a review summary as a top-level PR comment.
    pub async fn post_review_summary(&self, summary: &str, findings_count: usize) -> Result<Value, String> {
        let url = self.api_url(&format!("/pulls/{}/reviews", self.pr_number));

        let payload = json!({
            "body": format!(
                "# crabjar-review\n\n{}\n\n**{} issues found.**",
                summary, findings_count
            ),
            "event": "COMMENT",
        });

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let status = response.status();
        let body = response.json::<Value>().await.map_err(|e| format!("JSON parse failed: {}", e))?;

        if !status.is_success() {
            return Err(format!("GitHub API error {}: {:?}", status, body));
        }

        Ok(body)
    }

    /// Post all findings as review comments.
    pub async fn post_all_findings(
        &self,
        findings: &[Value],
    ) -> Result<Vec<Value>, String> {
        let results = Vec::new();

        for finding in findings {
            let file = finding["file"].as_str().unwrap_or("");
            let line = finding["line"].as_u64().unwrap_or(0) as u32;
            let description = finding["description"].as_str().unwrap_or("");
            let severity = finding["severity"].as_str().unwrap_or("minor");

            if file.is_empty() || line == 0 {
                continue;
            }

            if let Err(_e) = self.post_review_comment(file, line, description, severity).await {
                eprintln!("Warning: failed to post comment on {}:{}", file, line);
            } else {
                // success
            }
        }

        Ok(results)
    }
}

fn build_api_url(owner: &str, repo: &str, path: &str) -> String {
    format!("https://api.github.com/repos/{}/{}{}", owner, repo, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_api_url() {
        assert_eq!(
            build_api_url("owner", "repo", "/pulls/42/comments"),
            "https://api.github.com/repos/owner/repo/pulls/42/comments"
        );
    }
}
