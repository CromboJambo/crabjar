use crabjar_durable_objects::{CheckpointConfig, CheckpointStore};

fn main() {
    let config = CheckpointConfig::new("/tmp/test_checkpoints.db");
    let store = CheckpointStore::open(&config.db_path).unwrap();
    
    // Create a checkpoint
    let state = serde_json::json!({"step": 42, "data": vec![1, 2, 3]});
    let id = store.create_checkpoint("task-1", state.clone(), None).unwrap();
    println!("Checkpoint created: {}", id);
    
    // Later, resume from last checkpoint
    if let Some(cp) = store.get_latest_checkpoint("task-1").unwrap() {
        println!("Latest checkpoint ID: {}", cp.id);
        println!("State: {}", cp.state);
    }
    
    // Prune old checkpoints
    store.prune_checkpoints().unwrap();
    println!("Pruned old checkpoints");
}
