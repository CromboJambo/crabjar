//! phone-mcp: Unified phone MCP server merging KDE Connect + SSH file access.
//!
//! Presents a single "phone" interface to agents — no need to know whether
//! to use KDE Connect (wireless) or SSH (tethered) for a given operation.
//!
//! Tools:
//!   KDE Connect layer: list_devices, share_file, mount_device, get_mount_point, ping_device, send_message
//!   SSH layer:         list_files, phone_read_file, phone_write_file, download_file, upload_file, exec_command, ping_phone

use rmcp::{handler::server::wrapper::Parameters, schemars, tool, tool_router};
use serde_json::json;

// ─── Request Types ──────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct EmptyRequest {}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DeviceRequest {
    /// KDE Connect device ID or name
    pub device: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ShareRequest {
    /// KDE Connect device ID or name
    pub device: String,
    /// Absolute path to file on this machine
    pub path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MessageRequest {
    /// KDE Connect device ID or name
    pub device: String,
    /// Text message to send
    pub message: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListRequest {
    /// Directory path on phone (relative to ~)
    #[serde(default = "default_phone_path")]
    pub path: String,
}

fn default_phone_path() -> String { ".".to_string() }

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FileRequest {
    /// File path on phone (relative to ~)
    pub path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WriteRequest {
    /// File path on phone (relative to ~)
    pub path: String,
    /// Content to write
    pub content: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TransferRequest {
    /// Source path (phone-relative for download, local absolute for upload)
    pub source: String,
    /// Destination path (local absolute for download, phone-relative for upload)
    pub destination: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ExecRequest {
    /// Command to run on the phone via SSH
    pub command: String,
    /// Timeout in seconds (default: 30)
    #[serde(default = "default_exec_timeout")]
    pub timeout: u64,
}

fn default_exec_timeout() -> u64 { 30 }

// ─── Server ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PhoneServer;

impl PhoneServer {
    fn ssh_args(&self) -> Vec<String> {
        vec![
            "-o".to_string(), "StrictHostKeyChecking=no".to_string(),
            "-o".to_string(), "ConnectTimeout=5".to_string(),
            "crombo@192.168.1.42".to_string(),
        ]
    }

    fn run_ssh(&self, command: &str) -> anyhow::Result<String> {
        let mut args = self.ssh_args();
        args.push(command.to_string());

        let output = std::process::Command::new("ssh")
            .args(&args)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(stdout)
        } else {
            Err(anyhow::anyhow!("SSH command failed (exit {}): {}", output.status.code().unwrap_or(-1), stderr))
        }
    }

    fn run_scp(&self, source: &str, destination: &str) -> anyhow::Result<String> {
        let args = vec![
            "-o".to_string(), "StrictHostKeyChecking=no".to_string(),
            "-o".to_string(), "ConnectTimeout=5".to_string(),
        ];

        // For download: phone:path local:path
        // For upload:   local:path phone:path
        let mut cmd_args = vec![];
        if destination.starts_with('/') {
            // Download: source is phone path
            cmd_args.push(format!("crombo@192.168.1.42:{}", source));
            cmd_args.push(destination.to_string());
        } else {
            // Upload: destination is phone path
            cmd_args.push(source.to_string());
            cmd_args.push(format!("crombo@192.168.1.42:{}", destination));
        }

        let mut full_args = args;
        full_args.extend(cmd_args);

        let output = std::process::Command::new("scp")
            .args(&full_args)
            .output()?;

        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if output.status.success() {
            Ok(stderr)
        } else {
            Err(anyhow::anyhow!("SCP failed: {}", stderr))
        }
    }

    fn kdeconnect_cmd(&self, args: &[&str]) -> anyhow::Result<String> {
        let mut cmd = std::process::Command::new("kdeconnect-cli");
        for arg in args {
            cmd.arg(arg);
        }
        let output = cmd.output()?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(stdout)
        } else {
            Err(anyhow::anyhow!("kdeconnect-cli failed: {}", stderr))
        }
    }
}

#[tool_router(server_handler)]
impl PhoneServer {
    // ─── KDE Connect Tools ────────────────────────────────────────────────

    #[tool(description = "List all available KDE Connect devices")]
    fn list_devices(&self, _params: Parameters<EmptyRequest>) -> String {
        match self.kdeconnect_cmd(&["-l"]) {
            Ok(output) => serde_json::to_string(&json!({
                "devices": output.trim().lines().collect::<Vec<_>>(),
                "count": output.trim().lines().count()
            })).unwrap_or_default(),
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Share a file or URL to a KDE Connect device. Path is absolute path on this machine.")]
    fn share_file(&self, params: Parameters<ShareRequest>) -> String {
        let req = params.0;
        match self.kdeconnect_cmd(&["-i", &req.device, "-t", "share", "--path", &req.path]) {
            Ok(output) => format!("Shared to {}: {}", req.device, output.trim()),
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Mount the filesystem of a KDE Connect device. Returns mount point.")]
    fn mount_device(&self, params: Parameters<DeviceRequest>) -> String {
        let req = params.0;
        match self.kdeconnect_cmd(&["-i", &req.device, "-t", "mount"]) {
            Ok(output) => serde_json::to_string(&json!({
                "status": "mounted",
                "device": req.device,
                "mount_point": output.trim()
            })).unwrap_or_default(),
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Get the current mount point for a KDE Connect device")]
    fn get_mount_point(&self, params: Parameters<DeviceRequest>) -> String {
        let req = params.0;
        match self.kdeconnect_cmd(&["-i", &req.device, "-t", "mount-point"]) {
            Ok(output) => serde_json::to_string(&json!({
                "device": req.device,
                "mount_point": output.trim()
            })).unwrap_or_default(),
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Ping a KDE Connect device to check if it's reachable")]
    fn ping_device(&self, params: Parameters<DeviceRequest>) -> String {
        let req = params.0;
        match self.kdeconnect_cmd(&["-i", &req.device, "-t", "ping"]) {
            Ok(output) => serde_json::to_string(&json!({
                "device": req.device,
                "reachable": true,
                "response": output.trim()
            })).unwrap_or_default(),
            Err(e) => serde_json::to_string(&json!({
                "device": req.device,
                "reachable": false,
                "error": e.to_string()
            })).unwrap_or_default(),
        }
    }

    #[tool(description = "Send a text message to a KDE Connect device")]
    fn send_message(&self, params: Parameters<MessageRequest>) -> String {
        let req = params.0;
        match self.kdeconnect_cmd(&["-i", &req.device, "-t", "message", "--text", &req.message]) {
            Ok(output) => format!("Message sent to {}: {}", req.device, output.trim()),
            Err(e) => format!("Error: {}", e),
        }
    }

    // ─── SSH Tools ────────────────────────────────────────────────────────

    #[tool(description = "List files in a directory on the phone. Path is relative to phone home (~).")]
    fn list_files(&self, params: Parameters<ListRequest>) -> String {
        let req = params.0;
        match self.run_ssh(&format!("ls -la {}", req.path)) {
            Ok(output) => serde_json::to_string(&json!({
                "path": req.path,
                "files": output.trim().lines().collect::<Vec<_>>()
            })).unwrap_or_default(),
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Read a file from the phone. Path is relative to phone home (~). Returns file content as text.")]
    fn phone_read_file(&self, params: Parameters<FileRequest>) -> String {
        let req = params.0;
        match self.run_ssh(&format!("cat {}", req.path)) {
            Ok(output) => serde_json::to_string(&json!({
                "path": req.path,
                "content": output
            })).unwrap_or_default(),
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Write content to a file on the phone. Path is relative to phone home (~).")]
    fn phone_write_file(&self, params: Parameters<WriteRequest>) -> String {
        let req = params.0;
        // Use base64 to avoid escaping issues
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let encoded = STANDARD.encode(req.content.as_bytes());
        match self.run_ssh(&format!(
            "mkdir -p $(dirname {}) && echo {} | base64 -d > {}",
            req.path, encoded, req.path
        )) {
            Ok(_) => serde_json::to_string(&json!({
                "status": "success",
                "path": req.path,
                "bytes_written": req.content.len()
            })).unwrap_or_default(),
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Download a file from the phone to this machine. Source is on phone (~ relative), destination is local absolute path.")]
    fn download_file(&self, params: Parameters<TransferRequest>) -> String {
        let req = params.0;
        match self.run_scp(&req.source, &req.destination) {
            Ok(_) => serde_json::to_string(&json!({
                "status": "success",
                "from": req.source,
                "to": req.destination
            })).unwrap_or_default(),
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Upload a file from this machine to the phone. Source is local absolute path, destination is on phone (~ relative).")]
    fn upload_file(&self, params: Parameters<TransferRequest>) -> String {
        let req = params.0;
        match self.run_scp(&req.source, &req.destination) {
            Ok(_) => serde_json::to_string(&json!({
                "status": "success",
                "from": req.source,
                "to": req.destination
            })).unwrap_or_default(),
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Run a command on the phone via SSH. Returns stdout and stderr.")]
    fn exec_command(&self, params: Parameters<ExecRequest>) -> String {
        let req = params.0;
        match self.run_ssh(&req.command) {
            Ok(output) => serde_json::to_string(&json!({
                "output": output,
                "exit_code": 0
            })).unwrap_or_default(),
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Check if the phone is reachable via SSH")]
    fn ping_phone(&self, _params: Parameters<EmptyRequest>) -> String {
        match self.run_ssh("echo pong") {
            Ok(_) => serde_json::to_string(&json!({
                "reachable": true
            })).unwrap_or_default(),
            Err(e) => serde_json::to_string(&json!({
                "reachable": false,
                "error": e.to_string()
            })).unwrap_or_default(),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = PhoneServer;
    let running = rmcp::serve_server(server, (tokio::io::stdin(), tokio::io::stdout())).await?;
    running.waiting().await?;
    Ok(())
}
