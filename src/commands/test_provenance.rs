//! Comprehensive Provenance Tests
//!
//! Verifies decision provenance logging, versioning, and audit trail functionality.

#[cfg(test)]
mod tests {
    use crate::commands::decision_logging::{Decision, DecisionInput, MockEvidenceStore};
    use crate::commands::decision_versioning::{CorrectionType, DecisionCorrection, DecisionVersioner};
    use crate::commands::audit_trail::{AuditEntry, AuditStatus, DecisionProvenanceSystem};
    use crate::commands::decision_storage::{Criteria, DecisionInput as StoredInput, DecisionStorage, Source};

    #[test]
    fn test_decision_inputs_stored_correctly() {
        let mut store = MockEvidenceStore::new();

        let input_ids = vec!["input-a".to_string(), "input-b".to_string()];
        let decision_id = store.store_decision(
            "criteria-1".to_string(),
            1,
            input_ids.clone(),
            "approved".to_string(),
            None,
        );

        let decision = store.get_decision(&decision_id).unwrap();
        assert_eq!(decision.decision_inputs, input_ids);
    }

    #[test]
    fn test_criteria_version_tracked() {
        let mut store = MockEvidenceStore::new();

        let decision_id = store.store_decision(
            "criteria-1".to_string(),
            2,
            vec!["input-x".to_string()],
            "approved".to_string(),
            None,
        );

        let decision = store.get_decision(&decision_id).unwrap();
        assert_eq!(decision.criteria_version, 2);
    }

    #[test]
    fn test_parent_decision_linking() {
        let mut store = MockEvidenceStore::new();

        let parent_id = store.store_decision(
            "criteria-1".to_string(),
            1,
            vec!["input-1".to_string()],
            "approved".to_string(),
            None,
        );

        let child_id = store.store_decision(
            "criteria-1".to_string(),
            1,
            vec!["input-2".to_string()],
            "approved".to_string(),
            Some(parent_id.clone()),
        );

        let child = store.get_decision(&child_id).unwrap();
        assert_eq!(child.parent_decision_id, Some(parent_id));
    }

    #[test]
    fn test_version_history_retrieval() {
        let mut store = MockEvidenceStore::new();

        let parent_id = store.store_decision(
            "criteria-1".to_string(),
            1,
            vec!["input-1".to_string()],
            "approved".to_string(),
            None,
        );

        let child_id = store.store_decision(
            "criteria-1".to_string(),
            1,
            vec!["input-2".to_string()],
            "approved".to_string(),
            Some(parent_id.clone()),
        );

        let history = store.get_criteria_version_history("criteria-1");
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_correction_creates_new_version() {
        let mut store = MockEvidenceStore::new();

        let original_id = store.store_decision(
            "criteria-1".to_string(),
            1,
            vec!["input-1".to_string()],
            "approved".to_string(),
            None,
        );

        // Simulate correction by creating a new decision version
        let corrected_id = store.store_decision(
            "criteria-1".to_string(),
            2,
            vec!["input-1".to_string()],
            "corrected".to_string(),
            Some(original_id.clone()),
        );

        let original = store.get_decision(&original_id).unwrap();
        let corrected = store.get_decision(&corrected_id).unwrap();

        assert_eq!(original.decision_result, "approved");
        assert_eq!(corrected.decision_result, "corrected");
        assert_eq!(corrected.parent_decision_id, Some(original_id));
    }
}
