//! Crabjar Cross-Machine Deployment Tool
//!
//! Builds and deploys crabjar packages to target machines via SSH/SCP or
//! installs them locally. Supports dinit service management on remote targets.
//!
//! Usage: deploy <target> [package] [--service <name>]
//!   target: lappy, pixel9a, artix-box, ftw3, local
//!   package: crabjar (default), crabjar-conductor, vm-bridge, etc.
//!   --service: dinit service name to restart after deployment
//!
//! Replaces scripts/deploy.sh, deploy-ftw3.sh, deploy-conductor-ftw3.sh,
//! and deploy-vm-bridge-ftw3.sh with proper error handling and structured output.

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
    package: String,
    binary_name: String,
    version: String,
    install_dir: PathBuf,
    data_dir: PathBuf,
    service: Option<String>,
}

impl DeployConfig {
    fn new(target_name: &str, args: &[String]) -> Result<Self, String> {
        let target = TargetConfig::resolve(target_name)?;

        // Parse optional package name and --service flag
        let mut package = "crabjar".to_string();
        let mut service = None;
        let mut i = 0;
        while i < args.len() {
            if args[i] == "--service" && i + 1 < args.len() {
                service = Some(args[i + 1].clone());
                i += 2;
            } else if !args[i].starts_with("--") {
                package = args[i].clone();
                i += 1;
            } else {
                return Err(format!("Unknown flag: {}", args[i]));
            }
        }

        let binary_name = match package.as_str() {
            "crabjar" => "crabjar".to_string(),
            "crabjar-conductor" => "crabjar-conductor".to_string(),
            "vm-bridge" => "vm-bridge".to_string(),
            other => other.to_string(),
        };

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
            package,
            binary_name,
            version,
            install_dir,
            data_dir,
            service,
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

/// Build the release binary for a specific package.
fn build_release(package: &str) -> Result<PathBuf, String> {
    log_step(&format!("Building {} release binary...", package));

    let output = Command::new("cargo")
        .args(["build", "--release", "-p", package])
        .output()
        .map_err(|e| format!("Failed to run cargo build: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("cargo build failed:\n{}", stderr));
    }

    log_step("Build complete");
    Ok(PathBuf::from(format!("target/release/{}", package)))
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
        machine_id,
        now,
        machine_id,
        version,
        data_dir.display()
    )
}

/// Run an SSH command on the remote target.
fn ssh_exec(config: &DeployConfig, command: &str) -> Result<String, String> {
    let host = config.target.host.as_ref().unwrap();
    let ssh_key = config.target.ssh_key.as_ref().unwrap();

    let mut ssh_cmd = Command::new("ssh");
    ssh_cmd.arg("-i").arg(ssh_key);
    if let Some(p) = config.target.port {
        ssh_cmd.arg("-p").arg(p.to_string());
    }
    ssh_cmd.args([host, command]);

    let output = ssh_cmd
        .output()
        .map_err(|e| format!("Failed to run ssh: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "ssh failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Copy a file to the remote target via SCP.
fn scp_copy(config: &DeployConfig, local_path: &PathBuf, remote_path: &str) -> Result<(), String> {
    let host = config.target.host.as_ref().unwrap();
    let ssh_key = config.target.ssh_key.as_ref().unwrap();

    let mut scp_cmd = Command::new("scp");
    scp_cmd.arg("-i").arg(ssh_key);
    if let Some(p) = config.target.port {
        scp_cmd.arg("-P").arg(p.to_string());
    }
    scp_cmd
        .arg(local_path)
        .arg(&format!("{}:{}", host, remote_path));

    let output = scp_cmd
        .output()
        .map_err(|e| format!("Failed to run scp: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "scp failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(())
}

/// Deploy to a remote host via SSH/SCP.
fn deploy_remote(config: &DeployConfig, binary_path: &PathBuf) -> Result<(), String> {
    log_step(&format!(
        "Deploying {} to {}...",
        config.package, config.target.name
    ));

    // Create directory structure on remote
    log_step("Creating remote directories...");
    ssh_exec(
        config,
        &format!(
            "mkdir -p {} {}",
            config.install_dir.display(),
            config.data_dir.display()
        ),
    )?;

    // Copy binary to remote
    log_step("Copying binary...");
    let remote_path = format!("{}/{}", config.install_dir.display(), config.binary_name);
    scp_copy(config, binary_path, &remote_path)?;

    // Restart dinit service if specified
    if let Some(service) = &config.service {
        log_step(&format!("Restarting dinit service: {}", service));
        ssh_exec(config, &format!("sudo dinitctl stop {} || true", service))?;
        ssh_exec(config, &format!("sudo dinitctl start {}", service))?;
    }

    // Generate and deploy machine-specific configuration for main crabjar package
    if config.package == "crabjar" {
        log_step("Generating machine-specific configuration...");
        let machine_id = ssh_exec(config, "hostname")?;
        let config_content = generate_config(&machine_id, &config.version, &config.data_dir);

        // Write config to temp file and copy to remote
        let tmp_config = "/tmp/crabjar_deploy_config.toml";
        fs::write(tmp_config, &config_content)
            .map_err(|e| format!("Failed to write temp config: {}", e))?;

        let tmp_path = PathBuf::from(tmp_config);
        scp_copy(
            config,
            &tmp_path,
            &format!("{}/.crabjar_config.toml", config.install_dir.display()),
        )?;

        // Clean up temp file
        let _ = fs::remove_file(tmp_config);
    }

    log_step("Deployment complete");
    Ok(())
}

/// Deploy locally.
fn deploy_local(config: &DeployConfig, binary_path: &PathBuf) -> Result<(), String> {
    log_step(&format!(
        "Installing {} locally to {}...",
        config.package,
        config.install_dir.display()
    ));

    // Create directories
    fs::create_dir_all(&config.install_dir)
        .map_err(|e| format!("Failed to create install dir: {}", e))?;
    fs::create_dir_all(&config.data_dir)
        .map_err(|e| format!("Failed to create data dir: {}", e))?;

    // Copy binary
    log_step("Copying binary...");
    let dest = config.install_dir.join(&config.binary_name);
    fs::copy(binary_path, &dest).map_err(|e| format!("Failed to copy binary: {}", e))?;

    // Make executable
    let perms = std::fs::Permissions::from_mode(0o755);
    fs::set_permissions(&dest, perms).map_err(|e| format!("Failed to set permissions: {}", e))?;

    // Generate config for main crabjar package
    if config.package == "crabjar" {
        log_step("Generating machine-specific configuration...");
        let machine_id = run_command("hostname", &[])?;
        let config_content = generate_config(&machine_id, &config.version, &config.data_dir);

        fs::write(
            config.install_dir.join(".crabjar_config.toml"),
            &config_content,
        )
        .map_err(|e| format!("Failed to write config: {}", e))?;
    }

    log_step("Local deployment complete");
    Ok(())
}

/// Main deployment logic.
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: deploy <target> [package] [--service <name>]");
        eprintln!("Targets: lappy, pixel9a, artix-box, ftw3, local");
        eprintln!("Packages: crabjar (default), crabjar-conductor, vm-bridge");
        std::process::exit(1);
    }

    let target_name = &args[1];
    let extra_args = &args[2..];

    match run_deployment(target_name, extra_args) {
        Ok(_) => {
            println!("[deploy] Done!");
        }
        Err(e) => {
            eprintln!("[deploy] Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_deployment(target_name: &str, extra_args: &[String]) -> Result<(), String> {
    let config = DeployConfig::new(target_name, extra_args)?;

    log_step(&format!(
        "Deploying {} v{} to {}",
        config.package, config.version, config.target.name
    ));

    // Build release binary
    let binary_path = build_release(&config.package)?;

    // Deploy based on target
    if config.target.host.is_some() {
        deploy_remote(&config, &binary_path)?;
    } else {
        deploy_local(&config, &binary_path)?;
    }

    Ok(())
}
