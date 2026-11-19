//! LLM-powered inference for bug finding enrichment.
//!
//! Uses a local or remote LLM (via OpenAI-compatible API) to validate findings,
//! estimate severity, and generate remediation suggestions.

use crate::types::*;
use serde::Deserialize;

#[derive(Debug)]
pub struct Inferencer {
    api_base: String,
    model: String,
}

impl Inferencer {
    pub fn new(model: Option<String>) -> Self {
        // Default to local llama-server endpoint if not specified
        let api_base =
            std::env::var("LLM_API_BASE").unwrap_or_else(|_| "http://localhost:8081".to_string());
        let model_name = model.unwrap_or_else(|| "qwen2.5-coder-1.5b-instruct".to_string());

        Self {
            api_base,
            model: model_name,
        }
    }

    /// Enrich findings using LLM inference.
    pub async fn enrich(&self, findings: Vec<Finding>) -> Vec<Finding> {
        let mut enriched = Vec::new();

        for finding in findings {
            match self.infer_finding(&finding).await {
                Ok(enriched_finding) => enriched.push(enriched_finding),
                Err(e) => {
                    eprintln!("Inference failed for {}: {}", finding.id, e);
                    enriched.push(finding); // Keep original if inference fails
                }
            }
        }

        enriched
    }

    async fn infer_finding(&self, finding: &Finding) -> Result<Finding, crate::HunterError> {
        let mut result = finding.clone();

        // Build prompt for LLM
        let prompt = self.build_prompt(finding);

        // Call LLM API (OpenAI-compatible)
        let response = reqwest::Client::new()
            .post(format!("{}/v1/chat/completions", self.api_base))
            .json(&serde_json::json!({
                "model": self.model,
                "messages": [
                    {"role": "system", "content": "You are a security expert analyzing bug findings. Validate the finding, estimate confidence (0-1), and suggest better remediation if needed."},
                    {"role": "user", "content": prompt}
                ],
                "temperature": 0.3,
                "max_tokens": 500
            }))
            .send()
            .await?;

        let body: ChatResponse = response.json().await?;

        if let Some(choice) = body.choices.first() {
            let content = &choice.message.content;

            // Parse confidence from response
            if let Some(confidence) = self.extract_confidence(content) {
                result.confidence = Some(confidence);
            }

            // Parse improved remediation
            if let Some(remediation) = self.extract_remediation(content) {
                result.remediation = remediation;
            }

            result.inference_validated = true;
        }

        Ok(result)
    }

    fn build_prompt(&self, finding: &Finding) -> String {
        let evidence_json = serde_json::to_string_pretty(&finding.evidence).unwrap_or_default();

        format!(
            "Analyze this security finding:\n\n\
             Title: {}\n\
             Severity: {}\n\
             CWE: {}\n\
             Description: {}\n\
             Analyzer: {:?}\n\
             Evidence:\n{}\n\n\
             Questions:\n\
             1. Is this a real vulnerability or false positive?\n\
             2. What is your confidence level (0.0-1.0) that this is genuine?\n\
             3. Is the severity assessment correct? Adjust if needed.\n\
             4. Provide improved remediation steps if possible.\n\n\
             Format your response with clear sections for each question.",
            finding.title,
            finding.severity,
            finding.cwe.as_deref().unwrap_or("N/A"),
            finding.description,
            finding.analyzer,
            evidence_json
        )
    }

    fn extract_confidence(&self, content: &str) -> Option<f64> {
        // Look for confidence value in response
        let lines = content.lines();
        for line in lines {
            if line.to_lowercase().contains("confidence") {
                // Try to parse float from line
                let parts: Vec<&str> = line.split_whitespace().collect();
                for part in parts {
                    if let Ok(val) = part.parse::<f64>()
                        && (0.0..=1.0).contains(&val)
                    {
                        return Some(val);
                    }
                }
            }
        }
        None
    }

    fn extract_remediation(&self, content: &str) -> Option<String> {
        // Look for remediation section in response
        let lines = content.lines();
        let mut in_remediation = false;
        let mut remediation_lines = Vec::new();

        for line in lines {
            if line.to_lowercase().contains("remediation") || line.to_lowercase().contains("fix") {
                in_remediation = true;
                continue;
            }
            if in_remediation && !line.trim().is_empty() {
                remediation_lines.push(line);
            }
        }

        if !remediation_lines.is_empty() {
            Some(remediation_lines.join("\n"))
        } else {
            None
        }
    }
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
    content: String,
}
