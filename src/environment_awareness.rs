/// Environment awareness: proactive hardware and system state detection.
///
/// Detects available resources at session start and adapts agent behavior
/// (tool selection, parallelism, fallback strategies) based on actual capabilities.

use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};

/// System capability profile detected at runtime.
#[derive(Clone, Debug)]
pub struct EnvironmentProfile {
    pub gpu_available: bool,
    pub gpu_model: Option<String>,
    pub cpu_cores: usize,
    pub memory_gb: u64,
    pub disk_free_gb: u64,
    pub network_available: bool,
    pub inference_backend: String,
}

/// Proactive environment detector.
pub struct EnvironmentDetector {
    profile: Option<EnvironmentProfile>,
}

impl EnvironmentDetector {
    pub fn new() -> Self {
        Self { profile: None }
    }

    /// Detect system capabilities at session start.
    pub fn detect(&mut self) -> Result<EnvironmentProfile, String> {
        let gpu_available = self.check_gpu()?;
        let gpu_model = if gpu_available { self.get_gpu_model() } else { None };
        let cpu_cores = self.get_cpu_cores();
        let memory_gb = self.get_memory_gb();
        let disk_free_gb = self.get_disk_free_gb();
        let network_available = self.check_network();

        // Determine optimal inference backend based on available resources
        let inference_backend = if gpu_available {
            "gpu-accelerated".to_string()
        } else if cpu_cores >= 8 {
            "cpu-parallel".to_string()
        } else {
            "cpu-basic".to_string()
        };

        self.profile = Some(EnvironmentProfile {
            gpu_available,
            gpu_model,
            cpu_cores,
            memory_gb,
            disk_free_gb,
            network_available,
            inference_backend: inference_backend.clone(),
        });

        Ok((*self.profile.as_ref().unwrap()).clone())
    }

    fn check_gpu(&self) -> Result<bool, String> {
        // Check for NVIDIA GPU via nvidia-smi
        let output = std::process::Command::new("nvidia-smi")
            .arg("--query-gpu=name")
            .arg("--format=csv,noheader")
            .output();

        Ok(output.is_ok() && output.unwrap().status.success())
    }

    fn get_gpu_model(&self) -> Option<String> {
        let output = std::process::Command::new("nvidia-smi")
            .arg("--query-gpu=name")
            .arg("--format=csv,noheader")
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let model = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()?
            .trim()
            .to_string();

        Some(model)
    }

    fn get_cpu_cores(&self) -> usize {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    }

    fn get_memory_gb(&self) -> u64 {
        // Read from /proc/meminfo on Linux
        if let Ok(content) = fs::read_to_string("/proc/meminfo") {
            for line in content.lines() {
                if line.starts_with("MemTotal:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(kb) = parts[1].parse::<u64>() {
                            return kb / 1024 / 1024;
                        }
                    }
                }
            }
        }
        8 // Default assumption
    }

    fn get_disk_free_gb(&self) -> u64 {
        let home = std::env::var("HOME").unwrap_or_default();
        if let Ok(output) = std::process::Command::new("df")
            .arg("-BG")
            .arg(&home)
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    if let Ok(gb) = parts[3].trim_end_matches('G').parse::<u64>() {
                        return gb;
                    }
                }
            }
        }
        100 // Default assumption
    }

    fn check_network(&self) -> bool {
        std::process::Command::new("ping")
            .arg("-c")
            .arg("1")
            .arg("-W")
            .arg("2")
            .arg("8.8.8.8")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Generate adaptive tool selection recommendations based on environment.
    pub fn recommend_tools(&self, task_type: &str) -> Value {
        let profile = match &self.profile {
            Some(p) => p,
            None => return json!({"error": "Environment not detected yet"}),
        };

        match task_type {
            "inference" => {
                if profile.gpu_available {
                    json!({
                        "primary": "pesti-runner-gpu",
                        "fallback": "llama-cpp-cpu",
                        "reason": "GPU available for accelerated inference"
                    })
                } else {
                    json!({
                        "primary": "llama-cpp-cpu",
                        "fallback": "api-inference",
                        "reason": "No GPU detected, using CPU path"
                    })
                }
            }
            "training" => {
                if profile.gpu_available && profile.memory_gb >= 16 {
                    json!({
                        "primary": "ftw3-qlora-training",
                        "reason": "GPU + sufficient memory for QLoRA fine-tuning"
                    })
                } else {
                    json!({
                        "primary": "cpu-baseline-training",
                        "reason": "Limited resources, using CPU training path"
                    })
                }
            }
            _ => json!({"primary": "default", "reason": "No specific recommendation"})
        }
    }

    /// Check if environment has changed since last detection.
    pub fn check_for_changes(&mut self) -> Result<bool, String> {
        let new_profile = self.detect()?;
        match &self.profile {
            Some(old) => {
                Ok(new_profile.gpu_available != old.gpu_available ||
                   new_profile.cpu_cores != old.cpu_cores ||
                   new_profile.network_available != old.network_available)
            }
            None => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detector_initializes() {
        let detector = EnvironmentDetector::new();
        assert!(detector.profile.is_none());
    }
}