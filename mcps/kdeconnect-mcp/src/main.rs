//! KDE Connect MCP Server
//! Provides file access and device interaction via KDE Connect CLI.

use rmcp::{handler::server::wrapper::Parameters, schemars, tool, tool_router};
use std::process::Command;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct EmptyRequest {}

#[derive(Debug, Clone)]
pub struct KdeconnectServer;

fn run_kdeconnect(args: &[&str]) -> anyhow::Result<String> {
    // Get D-Bus session address from environment or default socket path
    let dbus_addr = std::env::var("DBUS_SESSION_BUS_ADDRESS").unwrap_or_else(|_| {
        format!("unix:path=/run/user/{}/bus", std::env::var("UID").unwrap_or("1000".to_string()))
    });

    let output = Command::new("kdeconnect-cli")
        .args(args)
        .env("DBUS_SESSION_BUS_ADDRESS", &dbus_addr)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("kdeconnect-cli failed: {}", stderr);
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[tool_router(server_handler)]
impl KdeconnectServer {
    #[tool(description = "List all available KDE Connect devices")]
    fn list_devices(&self, _params: Parameters<EmptyRequest>) -> String {
        match run_kdeconnect(&["--list-available", "--id-name-only"]) {
            Ok(result) => result,
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Share a file or URL to a KDE Connect device. Path is absolute path on this machine.")]
    fn share_file(&self, params: Parameters<ShareRequest>) -> String {
        let request = &params.0;
        match run_kdeconnect(&["-d", &request.device_id, "--share", &request.path]) {
            Ok(result) => result,
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Mount the filesystem of a KDE Connect device. Returns mount point.")]
    fn mount_device(&self, params: Parameters<DeviceRequest>) -> String {
        let request = &params.0;
        // First try to mount
        match run_kdeconnect(&["-d", &request.device_id, "--mount"]) {
            Ok(_) => {}
            Err(e) => return format!("Error mounting: {}", e),
        }

        // Then get mount point
        match run_kdeconnect(&["-d", &request.device_id, "--get-mount-point"]) {
            Ok(mount_point) => format!("Mounted at: {}", mount_point),
            Err(e) => format!("Error getting mount point: {}", e),
        }
    }

    #[tool(description = "Get the current mount point for a KDE Connect device")]
    fn get_mount_point(&self, params: Parameters<DeviceRequest>) -> String {
        let request = &params.0;
        match run_kdeconnect(&["-d", &request.device_id, "--get-mount-point"]) {
            Ok(mount_point) => mount_point,
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Ping a KDE Connect device to check if it's reachable")]
    fn ping_device(&self, params: Parameters<DeviceRequest>) -> String {
        let request = &params.0;
        match run_kdeconnect(&["-d", &request.device_id, "--ping"]) {
            Ok(result) => result,
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Send a text message to a KDE Connect device")]
    fn send_message(&self, params: Parameters<MessageRequest>) -> String {
        let request = &params.0;
        match run_kdeconnect(&["-d", &request.device_id, "--ping-msg", &request.message]) {
            Ok(result) => result,
            Err(e) => format!("Error: {}", e),
        }
    }
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DeviceRequest {
    pub device_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ShareRequest {
    pub device_id: String,
    pub path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MessageRequest {
    pub device_id: String,
    pub message: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = KdeconnectServer;
    let running = rmcp::serve_server(server, (tokio::io::stdin(), tokio::io::stdout())).await?;
    running.waiting().await?;
    Ok(())
}
