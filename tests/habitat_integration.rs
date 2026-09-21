//! Integration test: crabjar habitat contract producer → terrarium validator.
//!
//! Proves the full Phase 1 close-the-loop pipeline works end-to-end:
//! 1. crabjar generates a DAGR v3 habitat contract from real state
//! 2. terrarium validates the contract structure

use std::process::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn habitat_contract_produces_valid_dagr() {
    let tmp = TempDir::new().expect("temp dir");
    let contract_path = tmp.path().join("run.json");

    // crabjar habitat contract producer writes to .dagr/run.json by default.
    // We use the --out flag to redirect to our temp location.
    let crabjar = env!("CARGO_BIN_EXE_crabjar");
    let result = Command::new(crabjar)
        .args([
            "habitat", "contract",
            "--queue-path", "/dev/null",  // empty queue for test
            "--guard-db", ":memory:",     // in-memory guard db
            "--out", &contract_path.to_string_lossy(),
        ])
        .output()
        .expect("crabjar habitat contract");

    assert!(result.status.success(), "crabjar failed: {}", String::from_utf8_lossy(&result.stderr));

    // Verify the contract file was written
    let contract = fs::read_to_string(&contract_path)
        .expect("contract file should exist");

    // Basic structural validation (terrarium does full DAGR v3 validation)
    assert!(contract.contains("\"dagr\": 3"), "should be DAGR v3 format");
    assert!(contract.contains("\"tasks\""), "should have tasks array");
    assert!(contract.contains("\"projects\""), "should have projects array");

    // Now validate with terrarium (separate repo, use path directly)
    let terrarium = "/home/crombo/projects/terrarium/target/release/terrarium";
    if !std::path::Path::new(terrarium).exists() {
        eprintln!("NOTE: terrarium binary not found at {}, skipping validation", terrarium);
        return;
    }

    let result = Command::new(terrarium)
        .args(["validate", &contract_path.to_string_lossy()])
        .output()
        .expect("terrarium validate");

    assert!(result.status.success(), "terrarium validation failed: {}", String::from_utf8_lossy(&result.stderr));
    let output = String::from_utf8_lossy(&result.stdout);
    assert!(output.contains("OK"), "terrarium should report OK");
}
