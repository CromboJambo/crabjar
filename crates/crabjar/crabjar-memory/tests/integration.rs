//! Integration tests for Hermes-compatible memory system.
//!
//! Tests full round-trip with realistic Hermes memory file content,
//! verifying format compatibility and tool behavior end-to-end.

use std::fs;
use tempfile::tempdir;

use crabjar_memory::{MemoryTool, MemoryToolInput};

/// Realistic Hermes-style MEMORY.md content (similar to what agents actually store).
const HERMES_MEMORY_SAMPLE: &str = "Kev-9B decision engine profile created at ~/.hermes/profiles/kev-decisions/. Specialized bounded decision-making model running on ftw3:34159. Uses structured JSON criteria files from crabjar-structured-aux/specs/criteria/. Supports 7 decision types: approval, risk assessment, priority ranking, retry logic, resource allocation, MCP tool selection, compression. Profile includes SOUL.md with usage guidance and AGENT.md with integration documentation. Skill 'kev-decision-engine' created for cross-profile access.\n§\nOrchestrator: DavidAU/Qwen3.8-27B-TURBO-Fable (jambo:47493); Decision engine: Kev-9B (ftw3:34159, RTX 3070 Ti 8GB); Workers: Qwen2.5-Coder-1.5B (ftw3:8081), small models on lappy/artix-box.\n§\nFleet LLM migration (Oct 29, 2026): Unsloth Studio on ftw3 (8081, Qwen2.5-Coder-1.5B) and lappy (8082, Qwen3.8-4B). artix-box uses llama-server (8084, qwen2.5-3b-instruct) due to Vulkan GPU issue. sovnode runs llama-server (8083, Qwen3.8-2B-Distill). Podman on all nodes; crabjar seed container images on ftw3/artix-box/lappy (amd64), local binary on sovnode (arm64). Old llama-server services stopped on ftw3 and lappy.";

/// Realistic Hermes-style USER.md content.
const HERMES_USER_SAMPLE: &str = "User wants Pesti inference engine integrated with Unsloth Studio for backend swapping (CUDA via Pesti vs CPU/Vulkan via llama-server), enabling agent-driven compute placement decisions. Interested in building fleet-wide LLM scheduling/orchestration on top of Unsloth's model management APIs.\n§\nPrefers herdr + tuiwright workflow over Zed's GPUI ecosystem for agent-driven development. Wants to build their own TUI ecosystem using existing tools (helix, lazygit, yazi) managed through herdr/tuiwright rather than building custom UIs.";

#[test]
fn test_hermes_memory_roundtrip() {
    let dir = tempdir().unwrap();
    let memory_path = dir.path().join("MEMORY.md");

    // Write realistic Hermes-style content
    fs::write(&memory_path, HERMES_MEMORY_SAMPLE).unwrap();

    // Read back through the store
    let tool = MemoryTool::new(dir.path());
    let response = tool
        .execute(MemoryToolInput {
            target: "memory".to_string(),
            action: None,
            content: None,
            old_text: None,
            operations: Some(vec![]), // No-op batch to trigger read
        })
        .unwrap();

    assert!(response.success);
    assert_eq!(response.entries_count, 3);
    assert!(response.current_chars > 500);
    assert!(response.current_chars < response.max_chars);
}

#[test]
fn test_hermes_user_roundtrip() {
    let dir = tempdir().unwrap();
    let user_path = dir.path().join("USER.md");

    fs::write(&user_path, HERMES_USER_SAMPLE).unwrap();

    let tool = MemoryTool::new(dir.path());
    let response = tool
        .execute(MemoryToolInput {
            target: "user".to_string(),
            action: None,
            content: None,
            old_text: None,
            operations: Some(vec![]),
        })
        .unwrap();

    assert!(response.success);
    assert_eq!(response.entries_count, 2);
}

#[test]
fn test_add_entry_to_hermes_memory() {
    let dir = tempdir().unwrap();
    let memory_path = dir.path().join("MEMORY.md");
    fs::write(&memory_path, "First entry\n").unwrap();

    let tool = MemoryTool::new(dir.path());
    let response = tool
        .execute(MemoryToolInput {
            target: "memory".to_string(),
            action: Some(crabjar_memory::MemoryAction::Add),
            content: Some("Second entry about testing".to_string()),
            old_text: None,
            operations: None,
        })
        .unwrap();

    assert!(response.success);
    assert_eq!(response.entries_count, 2);

    // Verify file format is still Hermes-compatible (§ delimiter)
    let content = fs::read_to_string(&memory_path).unwrap();
    assert!(content.contains("§"));
    assert!(content.contains("First entry"));
    assert!(content.contains("Second entry about testing"));
}

#[test]
fn test_patch_entry_in_hermes_memory() {
    let dir = tempdir().unwrap();
    let memory_path = dir.path().join("MEMORY.md");
    fs::write(&memory_path, "Old configuration here\n§\nAnother entry\n").unwrap();

    let tool = MemoryTool::new(dir.path());
    let response = tool
        .execute(MemoryToolInput {
            target: "memory".to_string(),
            action: Some(crabjar_memory::MemoryAction::Patch),
            content: Some("New configuration".to_string()),
            old_text: Some("Old configuration".to_string()),
            operations: None,
        })
        .unwrap();

    assert!(response.success);
    let content = fs::read_to_string(&memory_path).unwrap();
    assert!(content.contains("New configuration here"));
    assert!(!content.contains("Old configuration"));
}

#[test]
fn test_remove_entry_from_hermes_memory() {
    let dir = tempdir().unwrap();
    let memory_path = dir.path().join("MEMORY.md");
    fs::write(&memory_path, "Entry to keep\n§\nEntry to remove\n").unwrap();

    let tool = MemoryTool::new(dir.path());
    let response = tool
        .execute(MemoryToolInput {
            target: "memory".to_string(),
            action: Some(crabjar_memory::MemoryAction::Remove),
            content: None,
            old_text: Some("Entry to remove".to_string()),
            operations: None,
        })
        .unwrap();

    assert!(response.success);
    assert_eq!(response.entries_count, 1);
    let content = fs::read_to_string(&memory_path).unwrap();
    assert!(content.contains("Entry to keep"));
    assert!(!content.contains("Entry to remove"));
}

#[test]
fn test_ambiguous_patch_returns_error() {
    let dir = tempdir().unwrap();
    let memory_path = dir.path().join("MEMORY.md");
    fs::write(&memory_path, "Shared text here\n§\nShared text there\n").unwrap();

    let tool = MemoryTool::new(dir.path());
    let response = tool
        .execute(MemoryToolInput {
            target: "memory".to_string(),
            action: Some(crabjar_memory::MemoryAction::Patch),
            content: Some("Replacement".to_string()),
            old_text: Some("Shared text".to_string()),
            operations: None,
        })
        .unwrap();

    assert!(!response.success);
    assert!(response.message.contains("Ambiguous") || response.message.contains("2"));
}

#[test]
fn test_batch_operations_atomic() {
    let dir = tempdir().unwrap();
    let memory_path = dir.path().join("MEMORY.md");
    fs::write(&memory_path, "Original entry\n").unwrap();

    let tool = MemoryTool::new(dir.path());
    let response = tool
        .execute(MemoryToolInput {
            target: "memory".to_string(),
            action: None,
            content: None,
            old_text: None,
            operations: Some(vec![
                crabjar_memory::MemoryOperation {
                    action: crabjar_memory::MemoryAction::Add,
                    content: Some("Batch entry 1".to_string()),
                    old_text: None,
                },
                crabjar_memory::MemoryOperation {
                    action: crabjar_memory::MemoryAction::Add,
                    content: Some("Batch entry 2".to_string()),
                    old_text: None,
                },
            ]),
        })
        .unwrap();

    assert!(response.success);
    assert_eq!(response.entries_count, 3);
}

#[test]
fn test_capacity_enforcement() {
    let dir = tempdir().unwrap();
    let memory_path = dir.path().join("MEMORY.md");

    // Fill close to the 2200 char limit
    let large_entry = "x".repeat(2100);
    fs::write(&memory_path, format!("{large_entry}\n")).unwrap();

    let tool = MemoryTool::new(dir.path());
    let response = tool
        .execute(MemoryToolInput {
            target: "memory".to_string(),
            action: Some(crabjar_memory::MemoryAction::Add),
            content: Some("y".repeat(200)), // Would push over limit
            old_text: None,
            operations: None,
        })
        .unwrap();

    assert!(!response.success);
    assert!(response.message.contains("Full") || response.message.contains("exceed"));
}

#[test]
fn test_user_target_separate_from_memory() {
    let dir = tempdir().unwrap();

    let tool = MemoryTool::new(dir.path());

    // Add to user target
    let _ = tool
        .execute(MemoryToolInput {
            target: "user".to_string(),
            action: Some(crabjar_memory::MemoryAction::Add),
            content: Some("User preference entry".to_string()),
            old_text: None,
            operations: None,
        })
        .unwrap();

    // Add to memory target
    let _ = tool
        .execute(MemoryToolInput {
            target: "memory".to_string(),
            action: Some(crabjar_memory::MemoryAction::Add),
            content: Some("Agent memory entry".to_string()),
            old_text: None,
            operations: None,
        })
        .unwrap();

    // Verify separate files
    let user_path = dir.path().join("USER.md");
    let memory_path = dir.path().join("MEMORY.md");
    assert!(user_path.exists());
    assert!(memory_path.exists());

    let user_content = fs::read_to_string(&user_path).unwrap();
    let memory_content = fs::read_to_string(&memory_path).unwrap();

    assert!(user_content.contains("User preference entry"));
    assert!(!user_content.contains("Agent memory entry"));
    assert!(memory_content.contains("Agent memory entry"));
    assert!(!memory_content.contains("User preference entry"));
}

#[test]
fn test_multiline_entry_preservation() {
    let dir = tempdir().unwrap();
    let memory_path = dir.path().join("MEMORY.md");

    let tool = MemoryTool::new(dir.path());
    let response = tool
        .execute(MemoryToolInput {
            target: "memory".to_string(),
            action: Some(crabjar_memory::MemoryAction::Add),
            content: Some("Line one\nLine two\nLine three".to_string()),
            old_text: None,
            operations: None,
        })
        .unwrap();

    assert!(response.success);
    let content = fs::read_to_string(&memory_path).unwrap();
    assert!(content.contains("Line one\nLine two\nLine three"));
}
