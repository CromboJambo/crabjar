# Durable Object Checkpointing Infrastructure

## Goal
Provide disk-backed checkpointing for worker execution contexts, enabling resume from last checkpoint on failure or restart.

## Architecture
- **CheckpointStore**: SQLite-backed metadata store tracking checkpoints by task ID
- **CheckpointWriter**: Writes serialized state to disk with atomicity guarantees
- **CheckpointReader**: Reads and validates checkpoint data
- **CheckpointConfig**: Configuration options for frequency and retention

## Design Decisions
- Use SQLite for metadata storage (consistent with existing conductor state store)
- File-based serialization for checkpoint payloads (JSON)
- Per-task checkpoint isolation via task_id prefix
- Configurable retention policy (count + age-based pruning)

## Implementation
New crate: `crabjar-durable-objects`

Public API:
```rust
let config = CheckpointConfig::new("/tmp/checkpoints.db");
let store = CheckpointStore::open(&config.db_path)?;

// Create checkpoint
let state = serde_json::json!({"step": 42});
let id = store.create_checkpoint("task-1", state, None)?;

// Resume from latest
if let Some(cp) = store.get_latest_checkpoint("task-1")? {
    println!("Checkpoint: {}", cp.id);
}
```

## Integration Points
- Worker lifecycle: checkpoint before/after command execution
- Dispatcher: resume logic on worker restart
- Kanban model: checkpoint at task boundaries via heartbeat metadata

## Configuration Options
- `checkpoint_interval`: Time between automatic checkpoints (default: 5min)
- `retention_count`: Number of checkpoints to keep per task (default: 10)
- `max_age`: Maximum age before pruning (default: 7 days)
- `compress`: Enable payload compression (default: true)