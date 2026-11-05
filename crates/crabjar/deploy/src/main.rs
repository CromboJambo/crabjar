//! Crabjar Cross-Machine Deployment Tool
//!
//! Builds the release binary and deploys it to a target machine via SSH/SCP
//! or installs it locally. Generates machine-specific configuration files.
//!
//! Replaces scripts/deploy.sh with proper error handling, structured output,
//! and execution tracking.

use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Target machine configuration.
struct TargetConfig {
    name: String,
    host: Option<String>,
    ssh_key: Option<PathBuf>,
    port: Option<u16>,
}

impl TargetConfig {
    fn resolve(target: &str) -> Result<Self, String> {
        let home = match env::var("HOME") {
            Ok(h) => PathBuf::from(h),
            Err(e) => return Err(format!("Failed to get HOME directory: {}", e)),
        };

        match target {
            "lappy" => Ok(Self {
                name: "lappy".to_string(),
                host: Some("crombo@lappy".to_string()),
                ssh_key: Some(home.join(".ssh/id_lab")),
                port: None,
            }),
            "pixel9a" => Ok(Self {
                name: "pixel9a".to_string(),
                host: Some("crombo@100.84.247.163".to_string()),
                ssh_key: Some(home.join(".ssh/id_pixel9a")),
                port: Some(8022),
            }),
            "artix-box" => Ok(Self {
                name: "artix-box".to_string(),
                host: Some("crombo@artix-box".to_string()),
                ssh_key: Some(home.join(".ssh/id_lab")),
                port: None,
            }),
            "ftw3" => Ok(Self {
                name: "ftw3".to_string(),
                host: Some("crombo@ftw3".to_string()),
                ssh_key: Some(home.join(".ssh/id_lab")),
                port: None,
            }),
            "local" => Ok(Self {
                name: "local".to_string(),
                host: None,
                ssh_key: None,
                port: None,
            }),
            other => Err(format!("Unknown target: {}", other)),
        }
    }
}

/// Deployment configuration.
struct DeployConfig {
    target: TargetConfig,
    version: String,
    install_dir: PathBuf,
    data_dir: PathBuf,
}

impl DeployConfig {
    fn new(target_name: &str) -> Result<Self, String> {
        let target = TargetConfig::resolve(target_name)?;

        // Determine version from git or environment
        let version = match env::var("CRABJAR_VERSION") {
            Ok(v) => v,
            Err(_) => {
                let output = Command::new("git")
                    .args(["describe", "--tags", "--always"])
                    .output()
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                    .unwrap_or_else(|_| "dev".to_string());
                if output.is_empty() {
                    "dev".to_string()
                } else {
                    output
                }
            }
        };

        let install_dir = PathBuf::from("/opt/crabjar");
        let data_dir = install_dir.join("data");

        Ok(Self {
            target,
            version,
            install_dir,
            data_dir,
        })
    }
}

/// Log a deployment step.
fn log_step(message: &str) {
    println!("[deploy] {}", message);
}

/// Run a command and return its output.
fn run_command(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run {}: {}", program, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("{} failed: {}", program, stderr.trim()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Build the release binary.
fn build_release() -> Result<PathBuf, String> {
    log_step("Building release binary...");

    let output = Command::new("cargo")
        .args(["build", "--release", "-p", "crabjar"])
        .output()
        .map_err(|e| format!("Failed to run cargo build: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("cargo build failed:\n{}", stderr));
    }

    log_step("Build complete");
    Ok(PathBuf::from("target/release/crabjar"))
}

/// Generate machine-specific configuration.
fn generate_config(machine_id: &str, version: &str, data_dir: &PathBuf) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    format!(
        "# Crabjar configuration for {}\n\
         # Generated: {}\n\n\
         [identity]\n\
         machine_id = \"{}\"\n\
         version = \"{}\"\n\n\
         [guard]\n\
         trust_layer = 0\n\
         auto_grant = false\n\n\
         [execution]\n\
         container_runtime = \"podman\"  # auto-detect: podman, docker, none\n\
         sandbox_user = \"crabjar-agent\"\n\
         timeout_seconds = 300\n\n\
         [telemetry]\n\
         enabled = true\n\
         log_level = \"info\"\n\
         output_dir = \"{}/logs\"\n",
        machine_id, now, machine_id, version, data_dir.display()
    )
}

/// Deploy to a remote host via SSH/SCP.
fn deploy_remote(config: &DeployConfig, binary_path: &PathBuf) -> Result<(), String> {
    let host = config.target.host.as_ref().unwrap();
    let ssh_key = config.target.ssh_key.as_ref().unwrap();

    log_step(&format!("Deploying to {}...", host));

    // Create directory structure on remote
    log_step("Creating remote directories...");
    let mut ssh_cmd = Command::new("ssh");
    ssh_cmd.arg("-i").arg(ssh_key);
    if let Some(p) = config.target.port {
        ssh_cmd.arg("-p").arg(p.to_string());
    }
    ssh_cmd.args([host, &format!("mkdir -p {} {}", config.install_dir.display(), config.data_dir.display())]);

    let output = ssh_cmd.output()
        .map_err(|e| format!("Failed to run ssh: {}", e))?;
    if !output.status.success() {
        return Err(format!("ssh mkdir failed: {}", String::from_utf8_lossy(&output.stderr).trim()));
    }

    // Copy binary to remote
    log_step("Copying binary...");
    let mut scp_cmd = Command::new("scp");
    scp_cmd.arg("-i").arg(ssh_key);
    if let Some(p) = config.target.port {
        scp_cmd.arg("-P").arg(p.to_string());
    }
    scp_cmd.arg(binary_path).arg(&format!("{}:{}", host, config.install_dir.display()));

    let output = scp_cmd.output()
        .map_err(|e| format!("Failed to run scp: {}", e))?;
    if !output.status.success() {
        return Err(format!("scp binary failed: {}", String::from_utf8_lossy(&output.stderr).trim()));
    }

    // Make binary executable on remote
    log_step("Setting permissions...");
    let mut ssh_cmd = Command::new("ssh");
    ssh_cmd.arg("-i").arg(ssh_key);
    if let Some(p) = config.target.port {
        ssh_cmd.arg("-p").arg(p.to_string());
    }
    ssh_cmd.args([host, &format!("chmod +x {}", config.install_dir.display())]);

    let output = ssh_cmd.output()
        .map_err(|e| format!("Failed to run ssh: {}", e))?;
    if !output.status.success() {
        return Err(format!("ssh chmod failed: {}", String::from_utf8_lossy(&output.stderr).trim()));
    }

    // Get machine ID from remote
    log_step("Generating machine-specific configuration...");
    let mut ssh_cmd = Command::new("ssh");
    ssh_cmd.arg("-i").arg(ssh_key);
    if let Some(p) = config.target.port {
        ssh_cmd.arg("-p").arg(p.to_string());
    }
    ssh_cmd.args([host, "hostname"]);

    let output = ssh_cmd.output()
        .map_err(|e| format!("Failed to run ssh: {}", e))?;
    if !output.status.success() {
        return Err(format!("ssh hostname failed: {}", String::from_utf8_lossy(&output.stderr).trim()));
    }
    let machine_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let config_content = generate_config(&machine_id, &config.version, &config.data_dir);

    // Write config to temp file and copy to remote
    let tmp_config = "/tmp/crabjar_deploy_config.toml";
    fs::write(tmp_config, &config_content)
        .map_err(|e| format!("Failed to write temp config: {}", e))?;

    let mut scp_cmd = Command::new("scp");
    scp_cmd.arg("-i").arg(ssh_key);
    if let Some(p) = config.target.port {
        scp_cmd.arg("-P").arg(p.to_string());
    }
    scp_cmd.args([tmp_config, &format!("{}:{}/.crabjar_config.toml", host, config.install_dir.display())]);

    let output = scp_cmd.output()
        .map_err(|e| format!("Failed to run scp: {}", e))?;
    if !output.status.success() {
        return Err(format!("scp config failed: {}", String::from_utf8_lossy(&output.stderr).trim()));
    }

    // Clean up temp file
    let _ = fs::remove_file(tmp_config);

    log_step("Deployment complete");
    Ok(())
}

/// Deploy locally.
fn deploy_local(config: &DeployConfig, binary_path: &PathBuf) -> Result<(), String> {
    log_step(&format!("Installing locally to {}...", config.install_dir.display()));

    // Create directories
    fs::create_dir_all(&config.install_dir)
        .map_err(|e| format!("Failed to create install dir: {}", e))?;
    fs::create_dir_all(&config.data_dir)
        .map_err(|e| format!("Failed to create data dir: {}", e))?;

    // Copy binary
    log_step("Copying binary...");
    fs::copy(binary_path, config.install_dir.join("crabjar"))
        .map_err(|e| format!("Failed to copy binary: {}", e))?;

    // Make executable
    let perms = std::fs::Permissions::from_mode(0o755);
    fs::set_permissions(config.install_dir.join("crabjar"), perms)
        .map_err(|e| format!("Failed to set permissions: {}", e))?;

    // Generate config
    log_step("Generating machine-specific configuration...");
    let machine_id = run_command("hostname", &[])?;
    let config_content = generate_config(&machine_id, &config.version, &config.data_dir);

    fs::write(config.install_dir.join(".crabjar_config.toml"), &config_content)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    log_step("Local deployment complete");
    Ok(())
}

/// Main deployment logic.
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: deploy <target>");
        eprintln!("Targets: lappy, pixel9a, artix-box, ftw3, local");
        std::process::exit(1);
    }

    let target_name = &args[1];

    match run_deployment(target_name) {
        Ok(_) => {
            println!("[deploy] Done!");
        }
        Err(e) => {
            eprintln!("[deploy] Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_deployment(target_name: &str) -> Result<(), String> {
    let config = DeployConfig::new(target_name)?;

    log_step(&format!("Deploying crabjar v{} to {}", config.version, config.target.name));

    // Build release binary
    let binary_path = build_release()?;

    // Deploy based on target
    if config.target.host.is_some() {
        deploy_remote(&config, &binary_path)?;
    } else {
        deploy_local(&config, &binary_path)?;
    }

    Ok(())
}