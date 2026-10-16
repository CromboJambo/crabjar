//! Integration tests for CrabJar Skill Scanner — comprehensive coverage for Phase 3
//! Tests all scanner checks with realistic skill packages and edge cases.

use crabjar_skill_scanner::{SkillScanner, Severity};
use std::fs;
use std::io::Write;
use tempfile::tempdir;

fn create_skill(dir: &std::path::Path, manifest: &str, files: &[(&str, &str)]) -> Result<(), String> {
    // Write manifest
    let manifest_path = dir.join("manifest.json");
    let mut f = fs::File::create(&manifest_path).map_err(|e| e.to_string())?;
    f.write_all(manifest.as_bytes()).map_err(|e| e.to_string())?;

    // Write additional files
    for (name, content) in files {
        let path = dir.join(name);
        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut f = fs::File::create(&path).map_err(|e| e.to_string())?;
        f.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn test_scan_clean_skill() {
    let scanner = SkillScanner::new();
    let dir = tempdir().unwrap();

    create_skill(dir.path(), r#"{"name": "clean-skill", "version": "1.0.0"}"#, &[
        ("index.js", "console.log('hello world');"),
        ("README.md", "# Clean Skill\nA simple skill."),
    ]).unwrap();

    let result = scanner.scan("clean-skill", "1.0.0", dir.path()).unwrap();
    assert!(result.passed, "Clean skill should pass: {}", result.summary());
    assert_eq!(result.files_scanned, 3); // manifest + index.js + README.md
}

#[test]
fn test_scan_missing_manifest() {
    let scanner = SkillScanner::new();
    let dir = tempdir().unwrap();

    fs::write(dir.path().join("index.js"), "console.log('hello');").unwrap();

    let result = scanner.scan("no-manifest", "1.0.0", dir.path()).unwrap();
    assert!(!result.passed);
    assert!(result.findings.iter().any(|f| f.category == "manifest"));
}

#[test]
fn test_scan_invalid_json_manifest() {
    let scanner = SkillScanner::new();
    let dir = tempdir().unwrap();

    create_skill(dir.path(), "{ invalid json }", &[
        ("index.js", "console.log('hello');"),
    ]).unwrap();

    let result = scanner.scan("bad-json", "1.0.0", dir.path()).unwrap();
    assert!(!result.passed);
    let manifest_finding = result.findings.iter().find(|f| f.category == "manifest");
    assert!(manifest_finding.is_some());
}

#[test]
fn test_scan_network_patterns() {
    let scanner = SkillScanner::new();
    let dir = tempdir().unwrap();

    create_skill(dir.path(), r#"{"name": "network-skill"}"#, &[
        ("index.js", "const res = fetch('https://api.example.com/data');"),
        ("utils.sh", "curl -s http://example.com/health"),
    ]).unwrap();

    let result = scanner.scan("network-skill", "1.0.0", dir.path()).unwrap();
    let network_findings = result.findings.iter().filter(|f| f.category == "network").count();
    assert!(network_findings >= 1, "Should detect at least one network pattern");
}

#[test]
fn test_scan_filesystem_write_patterns() {
    let scanner = SkillScanner::new();
    let dir = tempdir().unwrap();

    create_skill(dir.path(), r#"{"name": "fs-skill"}"#, &[
        ("index.js", "const fs = require('fs'); fs.writeFileSync('/tmp/output.txt', 'data');"),
    ]).unwrap();

    let result = scanner.scan("fs-skill", "1.0.0", dir.path()).unwrap();
    let fs_findings = result.findings.iter().filter(|f| f.category == "filesystem").count();
    assert!(fs_findings >= 1, "Should detect filesystem write patterns");
}

#[test]
fn test_scan_environment_access() {
    let scanner = SkillScanner::new();
    let dir = tempdir().unwrap();

    create_skill(dir.path(), r#"{"name": "env-skill"}"#, &[
        ("index.js", "const home = process.env.HOME;"),
        ("config.sh", "export PATH=$PATH:/usr/local/bin"),
    ]).unwrap();

    let result = scanner.scan("env-skill", "1.0.0", dir.path()).unwrap();
    let env_findings = result.findings.iter().filter(|f| f.category == "environment").count();
    assert!(env_findings >= 1, "Should detect environment variable access");
}

#[test]
fn test_scan_large_file() {
    let scanner = SkillScanner::new();
    let dir = tempdir().unwrap();

    // Create a file larger than 10MB limit
    let large_path = dir.path().join("large.bin");
    let mut f = fs::File::create(&large_path).unwrap();
    // Write 11MB of data (in chunks to avoid memory issues)
    for _ in 0..11 {
        let chunk = vec![0u8; 1024 * 1024];
        f.write_all(&chunk).unwrap();
    }

    create_skill(dir.path(), r#"{"name": "large-skill"}"#, &[
        ("index.js", "console.log('hello');"),
    ]).unwrap();

    let result = scanner.scan("large-skill", "1.0.0", dir.path()).unwrap();
    let size_findings = result.findings.iter().filter(|f| f.category == "file_size").count();
    assert!(size_findings >= 1, "Should detect file exceeding 10MB limit");
}

#[test]
fn test_scan_too_many_files() {
    let scanner = SkillScanner::new();
    let dir = tempdir().unwrap();

    create_skill(dir.path(), r#"{"name": "many-files"}"#, &[]).unwrap();

    // Create 105 files (exceeds 100 limit)
    for i in 0..105 {
        fs::write(dir.path().join(format!("file{}.js", i)), "console.log('hi');").unwrap();
    }

    let result = scanner.scan("many-files", "1.0.0", dir.path()).unwrap();
    let count_findings = result.findings.iter().filter(|f| f.category == "file_count").count();
    assert!(count_findings >= 1, "Should detect file count exceeding limit");
}

#[test]
fn test_scan_binary_files() {
    let scanner = SkillScanner::new();
    let dir = tempdir().unwrap();

    create_skill(dir.path(), r#"{"name": "binary-skill"}"#, &[
        ("index.js", "console.log('hello');"),
    ]).unwrap();

    // Create a binary file (non-text) with truly non-UTF8 bytes
    let binary_path = dir.path().join("data.bin");
    let mut f = fs::File::create(&binary_path).unwrap();
    // Use null bytes and high-bit values that aren't valid UTF-8
    let binary_data: Vec<u8> = vec![0x00, 0xFF, 0xFE, 0xFD, 0x80, 0x81];
    for _ in 0..200 {
        f.write_all(&binary_data).unwrap();
    }

    let result = scanner.scan("binary-skill", "1.0.0", dir.path()).unwrap();
    // Binary files > 1KB should be flagged
    let binary_findings = result.findings.iter().filter(|f| f.category == "binary_file").count();
    assert!(binary_findings >= 1, "Should detect non-text binary file");
}

#[test]
fn test_scan_with_subdirectories() {
    let scanner = SkillScanner::new();
    let dir = tempdir().unwrap();

    create_skill(dir.path(), r#"{"name": "nested-skill"}"#, &[
        ("src/index.js", "console.log('hello');"),
        ("lib/utils.js", "module.exports = { help: true };"),
        ("tests/test.js", "describe('test', () => {});"),
    ]).unwrap();

    let result = scanner.scan("nested-skill", "1.0.0", dir.path()).unwrap();
    assert!(result.passed);
    assert_eq!(result.files_scanned, 4); // manifest + 3 source files
}

#[test]
fn test_scan_empty_skill() {
    let scanner = SkillScanner::new();
    let dir = tempdir().unwrap();

    create_skill(dir.path(), r#"{"name": "empty-skill"}"#, &[]).unwrap();

    let result = scanner.scan("empty-skill", "1.0.0", dir.path()).unwrap();
    // Empty skill (just manifest) should pass
    assert!(result.passed);
}

#[test]
fn test_severity_levels() {
    let scanner = SkillScanner::new();
    let dir = tempdir().unwrap();

    // Create a skill with multiple issues
    create_skill(dir.path(), "{ bad json }", &[
        ("index.js", "fetch('https://example.com'); fs.writeFileSync('/tmp/x', 'y');"),
    ]).unwrap();

    let result = scanner.scan("multi-issue", "1.0.0", dir.path()).unwrap();
    
    // Should have findings of different severities
    let has_error = result.findings.iter().any(|f| f.severity == Severity::Error);
    let has_info = result.findings.iter().any(|f| f.severity == Severity::Info);
    
    assert!(has_error, "Should have error-level findings");
    assert!(has_info, "Should have info-level findings");
}

#[test]
fn test_scan_result_summary() {
    let scanner = SkillScanner::new();
    let dir = tempdir().unwrap();

    create_skill(dir.path(), r#"{"name": "summary-test"}"#, &[
        ("index.js", "console.log('hello');"),
    ]).unwrap();

    let result = scanner.scan("summary-test", "1.0.0", dir.path()).unwrap();
    
    // Summary should include file count and status
    let summary = result.summary();
    assert!(summary.contains("files") || summary.contains("findings"));
}

#[test]
fn test_scan_multiple_network_patterns() {
    let scanner = SkillScanner::new();
    let dir = tempdir().unwrap();

    create_skill(dir.path(), r#"{"name": "multi-network"}"#, &[
        ("script1.js", "fetch('https://api.example.com');"),
        ("script2.sh", "curl -s http://example.com/health"),
        ("script3.py", "import requests; requests.get('https://example.com')"),
    ]).unwrap();

    let result = scanner.scan("multi-network", "1.0.0", dir.path()).unwrap();
    // Should detect network patterns in multiple files
    let network_findings = result.findings.iter().filter(|f| f.category == "network").count();
    assert!(network_findings >= 2, "Should detect network patterns in multiple files");
}