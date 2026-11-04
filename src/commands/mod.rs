pub mod attempts;
pub mod approval;
pub mod checkpoint;
pub mod decide_backend;
pub mod decide_cache;
pub mod decide_criteria;
pub mod decide_db;
pub mod decide_learning;
pub mod decide_rules;
pub mod guard;
pub mod habitat;
pub mod semantic;
pub mod state;

// Command handler modules (extracted from main.rs)
pub mod backend;
pub mod dotfile;
pub mod ha;
pub mod knowledge;
pub mod metrics;
pub mod memory;
pub mod workspace;
pub mod fleet;
pub mod goals;
pub mod exec;
pub mod dinitctl;
pub mod podman;
pub mod slow_friend;