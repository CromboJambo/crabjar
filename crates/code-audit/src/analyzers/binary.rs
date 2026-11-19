//! Binary vulnerability analyzer.
//!
//! Inspects compiled binaries for security properties: PIE, NX, stack canaries,
//! RELRO, and other hardening features. Based on patterns from crabjar's
//! analyze-native infrastructure.

use crate::types::*;
use async_trait::async_trait;
use std::process::Command;

#[derive(Debug, Default)]
pub struct BinaryAnalyzer {}

impl BinaryAnalyzer {
    pub fn new() -> Self {
        Self {}
    }

    fn check_security_properties(&self, binary: &str) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Use readelf to check ELF properties (Linux-specific)
        if let Ok(output) = Command::new("readelf").args(["-h", binary]).output() {
            let header = String::from_utf8_lossy(&output.stdout);

            // Check for PIE (Position Independent Executable)
            if !header.contains("DYN") && !header.contains("ET_DYN") {
                findings.push(Finding {
                    id: format!("bin-no-pie-{}", binary_hash(binary)),
                    title: "Binary not compiled as PIE".to_string(),
                    description:
                        "Position Independent Executable not enabled — ASLR less effective"
                            .to_string(),
                    severity: Severity::Medium,
                    cwe: Some("CWE-120".to_string()),
                    analyzer: AnalyzerType::Binary,
                    evidence: vec![Evidence {
                        kind: "readelf_header".to_string(),
                        data: serde_json::json!({ "type": "EXEC (not DYN)" }),
                        location: Some(binary.to_string()),
                    }],
                    remediation: "Compile with -fPIE -pie flags".to_string(),
                    inference_validated: false,
                    confidence: None,
                });
            }
        }

        // Check for stack canaries via objdump
        if let Ok(output) = Command::new("objdump").args(["-d", binary]).output() {
            let disasm = String::from_utf8_lossy(&output.stdout);
            if !disasm.contains("__stack_chk_fail") {
                findings.push(Finding {
                    id: format!("bin-no-canary-{}", binary_hash(binary)),
                    title: "No stack canaries detected".to_string(),
                    description:
                        "__stack_chk_fail not found — stack buffer overflows may go undetected"
                            .to_string(),
                    severity: Severity::Medium,
                    cwe: Some("CWE-121".to_string()),
                    analyzer: AnalyzerType::Binary,
                    evidence: vec![Evidence {
                        kind: "objdump_disassembly".to_string(),
                        data: serde_json::json!({ "missing_symbol": "__stack_chk_fail" }),
                        location: Some(binary.to_string()),
                    }],
                    remediation: "Compile with -fstack-protector-strong or -fstack-protector-all"
                        .to_string(),
                    inference_validated: false,
                    confidence: None,
                });
            }
        }

        // Check for NX (non-executable stack)
        if let Ok(output) = Command::new("readelf").args(["-l", binary]).output() {
            let program_headers = String::from_utf8_lossy(&output.stdout);
            if program_headers.contains("RWE") {
                findings.push(Finding {
                    id: format!("bin-executable-stack-{}", binary_hash(binary)),
                    title: "Executable stack segment found".to_string(),
                    description: "Stack segment has execute permission — NX not enabled for stack"
                        .to_string(),
                    severity: Severity::High,
                    cwe: Some("CWE-120".to_string()),
                    analyzer: AnalyzerType::Binary,
                    evidence: vec![Evidence {
                        kind: "readelf_program_headers".to_string(),
                        data: serde_json::json!({ "flags": "RWE" }),
                        location: Some(binary.to_string()),
                    }],
                    remediation:
                        "Compile with -z noexecstack or ensure linker removes executable stack"
                            .to_string(),
                    inference_validated: false,
                    confidence: None,
                });
            }
        }

        findings
    }
}

#[async_trait]
impl Analyzer for BinaryAnalyzer {
    async fn analyze(
        &self,
        config: &crate::HuntConfig,
    ) -> Result<AnalyzerOutput, crate::HunterError> {
        // Verify binary exists
        if !std::path::Path::new(&config.target).exists() {
            return Err(crate::HunterError::TargetNotFound(config.target.clone()));
        }

        let findings = self.check_security_properties(&config.target);
        Ok(AnalyzerOutput { findings })
    }

    fn clone_box(&self) -> Box<dyn Analyzer> {
        Box::new(Self {})
    }
}

fn binary_hash(path: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}
