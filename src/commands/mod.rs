pub mod attempts;
pub mod approval;
pub mod checkpoint;
pub mod decide_backend;
pub mod decide_cache;
pub mod decide_criteria;
pub mod decide_db;
pub mod decide_rules;
pub mod guard;
pub mod habitat;
pub mod semantic;
pub mod state;

// Decision provenance query functions
pub fn list_decision_provenance() -> Result<(), Box<dyn std::error::Error>> {
    println!("Listing decision provenance records...");
    // TODO: Query database and display decision audit trail with parent links
    Ok(())
}

pub fn get_decision_provenance(decision_id: i64) -> Result<(), Box<dyn std::error::Error>> {
    println!("Getting provenance for decision {}", decision_id);
    // TODO: Retrieve specific decision's provenance chain
    Ok(())
}

pub fn list_decision_versions(decision_hash: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Listing versions for decision hash {}", decision_hash);
    // TODO: Show all versions of a decision (original + corrections)
    Ok(())
}
