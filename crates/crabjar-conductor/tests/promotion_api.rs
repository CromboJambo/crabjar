//! Integration test for evidence-based worker promotion API endpoints.
//! Demonstrates the full workflow: register worker → record performance → evaluate → promote.

use crabjar_conductor::{Conductor, ConductorConfig};
use std::sync::Arc;

#[tokio::test]
async fn test_promotion_api_workflow() {
    // Create conductor with in-memory DB (default)
    let config = ConductorConfig::default();
    let conductor = Arc::new(std::sync::Mutex::new(Conductor::new(config).unwrap()));

    // Register a worker
    let mut c = conductor.lock().unwrap();
    c.register_worker("test-worker-1".to_string(), vec!["shell".to_string()], Some("ssh-ed25519 AAAA...".to_string()))
        .unwrap();
    
    // Simulate performance: 50 successful tasks + 10 accurate decisions
    let pm = c.promotion_manager();
    for _ in 0..55 {
        pm.record_task_completion("test-worker-1", true).unwrap();
    }
    for _ in 0..12 {
        pm.record_decision("test-worker-1", true).unwrap();
    }

    // Evaluate eligibility
    let eval = pm.evaluate_promotion("test-worker-1", 1).unwrap();
    assert!(eval.eligible, "Worker should be eligible for promotion to supervisor rank");
    assert_eq!(eval.current_rank, 0);
    assert_eq!(eval.target_rank, 1);

    // Execute promotion
    let record = pm.execute_promotion("test-worker-1", 1, "Automated evidence-based promotion").unwrap();
    assert_eq!(record.from_rank, 0);
    assert_eq!(record.to_rank, 1);

    // Verify rank was updated
    assert_eq!(pm.get_worker_rank("test-worker-1").unwrap(), 1);

    // Check promotion history
    let history = pm.get_promotion_history("test-worker-1").unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].promotion_id, record.promotion_id);

    drop(c);

    println!("✅ Promotion API workflow test passed");
}
