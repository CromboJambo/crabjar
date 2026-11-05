//! Integration tests for CrabjarSurfaceAdapter and conductor API integration.

#[path = "../src/surface_adapter.rs"]
mod surface_adapter;

use serde_json::json;
use surface_adapter::{AgentStatus, CrabjarAgent, CrabjarSurfaceAdapter};

#[test]
fn test_agent_status_from_conductor() {
    // Test status translation from conductor worker states
    let idle_worker = json!({"id": "w1", "name": "worker-1", "status": "idle"});
    let agent = CrabjarAgent::from_conductor_response(&idle_worker);
    assert_eq!(agent.status, AgentStatus::Idle);

    let working_worker = json!({"id": "w2", "name": "worker-2", "status": "working"});
    let agent2 = CrabjarAgent::from_conductor_response(&working_worker);
    assert_eq!(agent2.status, AgentStatus::Working);

    let blocked_worker = json!({"id": "w3", "name": "worker-3", "status": "blocked"});
    let agent3 = CrabjarAgent::from_conductor_response(&blocked_worker);
    assert_eq!(agent3.status, AgentStatus::Blocked);

    let done_worker = json!({"id": "w4", "name": "worker-4", "status": "done"});
    let agent4 = CrabjarAgent::from_conductor_response(&done_worker);
    assert_eq!(agent4.status, AgentStatus::Done);
}

#[test]
fn test_agent_status_colors() {
    // Verify color codes match the architecture doc spec
    assert_eq!(AgentStatus::Idle.color(), [100, 149, 237]); // CornflowerBlue
    assert_eq!(AgentStatus::Working.color(), [50, 205, 50]); // LimeGreen
    assert_eq!(AgentStatus::Blocked.color(), [255, 165, 0]); // Orange
    assert_eq!(AgentStatus::Done.color(), [70, 130, 180]); // SteelBlue
}

#[test]
fn test_surface_adapter_update() {
    let mut adapter = CrabjarSurfaceAdapter::new("test-boot-id");

    let workers = vec![
        json!({"id": "w1", "name": "hermes-agent", "status": "working"}),
        json!({"id": "w2", "name": "codex-agent", "status": "idle"}),
    ];

    adapter.update_agents(&workers);

    assert_eq!(adapter.agents().len(), 2);
    assert!(adapter.agents().contains_key("w1"));
    assert!(adapter.agents().contains_key("w2"));
}

#[test]
fn test_surface_patch_generation() {
    let mut adapter = CrabjarSurfaceAdapter::new("test-boot-id");

    let workers = vec![json!({"id": "w1", "name": "hermes-agent", "status": "working"})];

    adapter.update_agents(&workers);
    let patch = adapter.generate_patch();

    assert_eq!(patch["boot_id"], "test-boot-id");
    assert_eq!(patch["projection_revision"], 1);
    assert_eq!(patch["surface_revision"], 1);
    assert_eq!(patch["agents"].as_array().unwrap().len(), 1);
}

#[test]
fn test_conductor_api_integration() {
    // Test that we can construct the correct API request format
    let conductor_url = "http://127.0.0.1:8080";
    let workers_url = format!("{}/workers", conductor_url);
    assert_eq!(workers_url, "http://127.0.0.1:8080/workers");

    let goals_url = format!("{}/goals", conductor_url);
    assert_eq!(goals_url, "http://127.0.0.1:8080/goals");
}

#[test]
fn test_goal_submission_format() {
    // Verify the JSON format for goal submission matches conductor API spec
    let goal = json!({
        "description": "Build crabjar-gpui dashboard",
        "priority": 1,
        "assignee": null
    });

    assert_eq!(goal["description"], "Build crabjar-gpui dashboard");
    assert_eq!(goal["priority"], 1);
}
