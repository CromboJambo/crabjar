//! SSH File MCP Server
//! Provides file access on remote devices via SSH/SFTP command-line tools.

use rmcp::{handler::server::wrapper::Parameters, schemars, tool, tool_router};
use std::process::Command;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct EmptyRequest {}

#[derive(Debug, Clone)]
pub struct SshFileServer {
    host: String,
    port: u16,
    user: String,
    key_path: String,
}

impl SshFileServer {
    fn new() -> Self {
        Self {
            host: "pixel-9a.taila95750.ts.net".to_string(),
            port: 8022,
            user: "crombo".to_string(),
            key_path: "/home/crombo/.ssh/id_pixel-9a".to_string(),
        }
    }

    fn ssh_args(&self) -> Vec<String> {
        vec![
            "-p".to_string(),
            self.port.to_string(),
            "-i".to_string(),
            self.key_path.clone(),
            "-o".to_string(),
            "ConnectTimeout=30".to_string(),
            "-o".to_string(),
            "StrictHostKeyChecking=no".to_string(),
            format!("{}@{}", self.user, self.host),
        ]
    }

    fn run_ssh(&self, command: &str) -> anyhow::Result<String> {
        let args = self.ssh_args();
        let mut cmd = Command::new("ssh");
        cmd.args(&args).arg(command);

        let output = cmd.output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("SSH command failed: {}", stderr);
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn run_scp(&self, source: &str, destination: &str) -> anyhow::Result<String> {
        let mut cmd = Command::new("scp");
        cmd.arg("-P").arg(self.port.to_string());
        cmd.arg("-i").arg(&self.key_path);
        cmd.arg("-o").arg("ConnectTimeout=30");
        cmd.arg("-o").arg("StrictHostKeyChecking=no");
        cmd.arg(source).arg(destination);

        let output = cmd.output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("SCP failed: {}", stderr);
        }
        Ok("Success".to_string())
    }
}

#[tool_router(server_handler)]
impl SshFileServer {
    #[tool(description = "List files in a directory on the phone. Path is relative to phone home (~).")]
    fn list_files(&self, params: Parameters<ListRequest>) -> String {
        let request = &params.0;
        let path = if request.path.starts_with("~/") || request.path == "~" {
            format!("/data/data/com.termux/files/home{}", &request.path[1..])
        } else {
            request.path.clone()
        };

        match self.run_ssh(&format!("ls -la {}", path)) {
            Ok(result) => result,
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Read a file from the phone. Path is relative to phone home (~). Returns file content as text.")]
    fn read_file(&self, params: Parameters<FileRequest>) -> String {
        let request = &params.0;
        let path = if request.path.starts_with("~/") || request.path == "~" {
            format!("/data/data/com.termux/files/home{}", &request.path[1..])
        } else {
            request.path.clone()
        };

        match self.run_ssh(&format!("cat {}", path)) {
            Ok(result) => result,
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Write content to a file on the phone. Path is relative to phone home (~).")]
    fn write_file(&self, params: Parameters<WriteRequest>) -> String {
        let request = &params.0;
        // Write to temp file locally, then SCP to phone
        let local_temp = "/tmp/ssh-file-mcp-write-tmp";
        match std::fs::write(local_temp, &request.content) {
            Ok(_) => {}
            Err(e) => return format!("Error writing temp file: {}", e),
        }

        let dest_path = if request.path.starts_with("~/") || request.path == "~" {
            format!("/data/data/com.termux/files/home{}", &request.path[1..])
        } else {
            request.path.clone()
        };

        match self.run_scp(local_temp, &dest_path) {
            Ok(result) => {
                std::fs::remove_file(local_temp).ok();
                result
            }
            Err(e) => {
                std::fs::remove_file(local_temp).ok();
                format!("Error: {}", e)
            }
        }
    }

    #[tool(description = "Download a file from the phone to this machine. Source is on phone (~ relative), destination is local absolute path.")]
    fn download_file(&self, params: Parameters<TransferRequest>) -> String {
        let request = &params.0;
        let src_path = if request.source.starts_with("~/") || request.source == "~" {
            format!("/data/data/com.termux/files/home{}", &request.source[1..])
        } else {
            request.source.clone()
        };

        match self.run_scp(&src_path, &request.destination) {
            Ok(result) => result,
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Upload a file from this machine to the phone. Source is local absolute path, destination is on phone (~ relative).")]
    fn upload_file(&self, params: Parameters<TransferRequest>) -> String {
        let request = &params.0;
        let dest_path = if request.destination.starts_with("~/") || request.destination == "~" {
            format!("/data/data/com.termux/files/home{}", &request.destination[1..])
        } else {
            request.destination.clone()
        };

        match self.run_scp(&request.source, &dest_path) {
            Ok(result) => result,
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Run a command on the phone via SSH. Returns stdout and stderr.")]
    fn exec_command(&self, params: Parameters<ExecRequest>) -> String {
        let request = &params.0;
        match self.run_ssh(&request.command) {
            Ok(result) => result,
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Check if the phone is reachable via SSH")]
    fn ping_phone(&self, _params: Parameters<EmptyRequest>) -> String {
        match self.run_ssh("echo 'Phone is reachable'") {
            Ok(result) => result,
            Err(e) => format!("Phone not reachable: {}", e),
        }
    }
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListRequest {
    pub path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FileRequest {
    pub path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WriteRequest {
    pub path: String,
    pub content: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TransferRequest {
    pub source: String,
    pub destination: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ExecRequest {
    pub command: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = SshFileServer::new();
    let running = rmcp::serve_server(server, (tokio::io::stdin(), tokio::io::stdout())).await?;
    running.waiting().await?;
    Ok(())
}
