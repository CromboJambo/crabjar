//! crabjar-unsloth-mcp: MCP server wrapping Unsloth Studio for training data management.
//!
//! Exposes tools for listing datasets, inspecting metadata, starting training runs,
//! checking status, and evaluating checkpoints via the Unsloth CLI.

use rmcp::{handler::server::wrapper::Parameters, schemars, tool, tool_router};
use std::process::Command;

const TRAINING_DATA_DIR: &str = "/home/crombo/training-data-dump";
const UNSLOTH_OUTPUTS_DIR: &str = "~/.unsloth/studio/outputs";

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListDatasetsRequest {
    /// Optional filter by extension (e.g., "jsonl", "json", "txt", "md"). Defaults to all.
    #[serde(default)]
    pub extension: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct InspectDatasetRequest {
    /// Filename of the dataset (e.g., "training_data.jsonl.cleaned").
    pub filename: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StartTrainingRequest {
    /// Base model to fine-tune (e.g., "unsloth/Qwen3.5-4B-Base").
    pub base_model: String,
    /// Training dataset filename from the dump directory.
    pub dataset: String,
    /// Number of training epochs (default: 1).
    #[serde(default = "default_epochs")]
    pub epochs: u32,
    /// Learning rate (default: 2e-5).
    #[serde(default = "default_learning_rate")]
    pub learning_rate: f64,
    /// Batch size (default: 2).
    #[serde(default = "default_batch_size")]
    pub batch_size: u32,
}

fn default_epochs() -> u32 {
    1
}
fn default_learning_rate() -> f64 {
    2e-5
}
fn default_batch_size() -> u32 {
    2
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TrainingStatusRequest {
    /// Optional run identifier. If omitted, shows all recent runs.
    #[serde(default)]
    pub run_id: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct EvaluateCheckpointRequest {
    /// Path to checkpoint or LoRA adapter.
    pub checkpoint_path: String,
    /// Evaluation task (default: "hellaswag").
    #[serde(default = "default_eval_task")]
    pub task: String,
}

fn default_eval_task() -> String {
    "hellaswag".to_string()
}

#[derive(Debug, Clone)]
pub struct UnslothServer;

fn run_cmd(cmd: &str) -> anyhow::Result<serde_json::Value> {
    let output = Command::new("bash").args(["-c", cmd]).output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        // Try to parse as JSON, fall back to plain text
        match serde_json::from_str(&stdout) {
            Ok(val) => Ok(val),
            Err(_) => Ok(serde_json::json!({ "output": stdout })),
        }
    } else {
        Ok(serde_json::json!({
            "success": false,
            "error": stderr,
            "exit_code": output.status.code()
        }))
    }
}

#[tool_router(server_handler)]
impl UnslothServer {
    #[tool(
        description = "List available training datasets in the dump directory. Optionally filter by file extension (jsonl, json, txt, md). Returns dataset names with line counts and sizes."
    )]
    fn list_datasets(&self, params: Parameters<ListDatasetsRequest>) -> String {
        let req = params.0;

        let ext_filter = match &req.extension {
            Some(ext) => format!("*.{}.*", ext),
            None => String::from("*.cleaned"),
        };

        let cmd = format!(
            "cd {} && find . -maxdepth 1 -name '{}' -type f | sort | while read f; do echo \"=== $f ===\"; wc -l \"$f\" 2>/dev/null || echo '(not text)'; du -h \"$f\" 2>/dev/null | cut -f1; done",
            TRAINING_DATA_DIR, ext_filter
        );

        match run_cmd(&cmd) {
            Ok(result) => serde_json::to_string(&result)
                .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }

    #[tool(
        description = "Inspect a specific training dataset. Returns metadata including line count, file size, format detection, and a sample of the first few entries."
    )]
    fn inspect_dataset(&self, params: Parameters<InspectDatasetRequest>) -> String {
        let req = params.0;

        let cmd = format!(
            "cd {} && echo '=== Metadata ===' && ls -lh '{}' 2>&1 && echo '=== Line Count ===' && wc -l '{}' 2>&1 && echo '=== Format Detection ===' && file '{}' 2>&1 && echo '=== Sample (first 3 lines) ===' && head -n 3 '{}' 2>&1",
            TRAINING_DATA_DIR, req.filename, req.filename, req.filename, req.filename
        );

        match run_cmd(&cmd) {
            Ok(result) => serde_json::to_string(&result)
                .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }

    #[tool(
        description = "Start an Unsloth fine-tuning job. Requires base_model and dataset filename from the dump directory. Optional params: epochs (default 1), learning_rate (default 2e-5), batch_size (default 2). Returns training run info including output directory."
    )]
    fn start_training(&self, params: Parameters<StartTrainingRequest>) -> String {
        let req = params.0;

        // Unsloth CLI uses `unsloth train` with various flags
        let dataset_path = format!("{}/{}", TRAINING_DATA_DIR, req.dataset);
        let cmd = format!(
            "unsloth train --model {} --data '{}' --epochs {} --learning-rate {} --batch-size {}",
            req.base_model, dataset_path, req.epochs, req.learning_rate, req.batch_size
        );

        match run_cmd(&cmd) {
            Ok(result) => serde_json::to_string(&result)
                .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }

    #[tool(
        description = "Check status of Unsloth training runs. Optionally specify a run_id to check a specific run. Returns run state, progress, and recent logs."
    )]
    fn training_status(&self, params: Parameters<TrainingStatusRequest>) -> String {
        let req = params.0;

        match &req.run_id {
            Some(run_id) => {
                let cmd = format!("unsloth train --status {}", run_id);
                match run_cmd(&cmd) {
                    Ok(result) => serde_json::to_string(&result)
                        .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                    Err(e) => format!("{{\"error\": \"{}\"}}", e),
                }
            }
            None => {
                // List recent runs from outputs directory
                let cmd = format!("ls -lt {} | head -10", UNSLOTH_OUTPUTS_DIR);
                match run_cmd(&cmd) {
                    Ok(result) => serde_json::to_string(&result)
                        .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
                    Err(e) => format!("{{\"error\": \"{}\"}}", e),
                }
            }
        }
    }

    #[tool(
        description = "List available model checkpoints from Unsloth training runs. Shows checkpoint directories with their base models and timestamps."
    )]
    fn list_checkpoints(&self) -> String {
        let cmd = format!(
            "find {} -maxdepth 2 -name 'adapter_model.safetensors' -type f 2>/dev/null | head -20",
            UNSLOTH_OUTPUTS_DIR
        );

        match run_cmd(&cmd) {
            Ok(result) => serde_json::to_string(&result)
                .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }

    #[tool(
        description = "Evaluate a model checkpoint using lm-eval-harness. Requires checkpoint_path and optionally task (default: hellaswag). Returns evaluation metrics."
    )]
    fn evaluate_checkpoint(&self, params: Parameters<EvaluateCheckpointRequest>) -> String {
        let req = params.0;

        let cmd = format!(
            "unsloth eval --checkpoint '{}' --task {}",
            req.checkpoint_path, req.task
        );

        match run_cmd(&cmd) {
            Ok(result) => serde_json::to_string(&result)
                .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = UnslothServer;
    let running = rmcp::serve_server(server, (tokio::io::stdin(), tokio::io::stdout())).await?;
    running.waiting().await?;
    Ok(())
}
