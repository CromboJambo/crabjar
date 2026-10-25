//! Comprehensive Integration Tests for Crabjar Approval Workflow
//!
//! Validates all core components work together correctly.

#[cfg(test)]
mod tests {
    use crate::commands::impact_assessment::{assess_impact, ImpactLevel, ServiceCriticality};
    use crate::commands::approval_models::{ApprovalQueue, ApprovalRequest, ApprovalStatus, ChangeRequest, ImpactLevel as ModelImpact};
    use crate::commands::approval_service::{ApprovalQueueService, ChangeRequest as ServiceChange, ChangeStatus};
    use crate::commands::decision_versioning::{create_decision_version, get_decision_history, CorrectionType};
    use crate::commands::audit_trail::{DecisionProvenanceSystem, AuditEntry};

    // Impact Assessment Tests
    #[test]
    fn test_impact_low_classification() {
        let result = assess_impact(5, vec![ServiceCriticality::Standard], "config", false);
        assert_eq!(result.level, ImpactLevel::Low);
    }

    #[test]
    fn test_impact_medium_classification() {
        let result = assess_impact(50, vec![ServiceCriticality::Standard], "config", false);
        assert_eq!(result.level, ImpactLevel::Medium);
    }

    #[test]
    fn test_impact_high_classification() {
        let result = assess_impact(200, vec![ServiceCriticality::Standard], "config", false);
        assert_eq!(result.level, ImpactLevel::High);
    }

    // Approval Queue Tests
    #[test]
    fn test_approval_queue_add_change() {
        let mut service = ApprovalQueueService::new();
        let change = ServiceChange {
            id: "test-1".to_string(),
            title: "Test Change".to_string(),
            summary: "Testing approval queue".to_string(),
            submitter_id: "user1".to_string(),
            submitted_at: 0,
            status: ChangeStatus::Pending,
            affected_projects: vec!["test-project".to_string()],
            required_approvals: vec!["approver1".to_string()],
            min_approvals_required: 1,
            approvals_received: 0,
            rejection_reason: None,
        };

        let id = service.add_change(change);
        assert_eq!(id, "test-1");
    }

    #[test]
    fn test_approval_queue_list_pending() {
        let mut service = ApprovalQueueService::new();
        let change = ServiceChange {
            id: "test-2".to_string(),
            title: "Test Change 2".to_string(),
            summary: "Testing pending list".to_string(),
            submitter_id: "user1".to_string(),
            submitted_at: 0,
            status: ChangeStatus::Pending,
            affected_projects: vec!["test-project".to_string()],
            required_approvals: vec!["approver1".to_string()],
            min_approvals_required: 1,
            approvals_received: 0,
            rejection_reason: None,
        };

        service.add_change(change);
        let pending = service.get_pending();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "test-2");
    }

    #[test]
    fn test_approval_queue_approve_change() {
        let mut service = ApprovalQueueService::new();
        let change = ServiceChange {
            id: "test-3".to_string(),
            title: "Test Change 3".to_string(),
            summary: "Testing approval".to_string(),
            submitter_id: "user1".to_string(),
            submitted_at: 0,
            status: ChangeStatus::Pending,
            affected_projects: vec!["test-project".to_string()],
            required_approvals: vec!["approver1".to_string()],
            min_approvals_required: 1,
            approvals_received: 0,
            rejection_reason: None,
        };

        service.add_change(change);
        let approved = service.approve_change("test-3", "approver1");
        assert!(approved);

        let pending = service.get_pending();
        assert_eq!(pending.len(), 0);
    }

    #[test]
    fn test_approval_queue_reject_change() {
        let mut service = ApprovalQueueService::new();
        let change = ServiceChange {
            id: "test-4".to_string(),
            title: "Test Change 4".to_string(),
            summary: "Testing rejection".to_string(),
            submitter_id: "user1".to_string(),
            submitted_at: 0,
            status: ChangeStatus::Pending,
            affected_projects: vec!["test-project".to_string()],
            required_approvals: vec!["approver1".to_string()],
            min_approvals_required: 1,
            approvals_received: 0,
            rejection_reason: None,
        };

        service.add_change(change);
        let rejected = service.reject_change("test-4", "approver1", "Not needed");
        assert!(rejected);

        let pending = service.get_pending();
        assert_eq!(pending.len(), 0);
    }

    // Decision Versioning Tests
    #[test]
    fn test_decision_version_creation() {
        let version_id = create_decision_version(
            "decision-1".to_string(),
            "Initial decision".to_string(),
            None,
            CorrectionType::None,
        );
        assert!(!version_id.is_empty());
    }

    #[test]
    fn test_decision_history_tracking() {
        let v1 = create_decision_version(
            "decision-2".to_string(),
            "Initial version".to_string(),
            None,
            CorrectionType::None,
        );

        let v2 = create_decision_version(
            "decision-2-correction".to_string(),
            "Corrected version".to_string(),
            Some(v1.clone()),
            CorrectionType::Factual,
        );

        assert_ne!(v1, v2);
    }

    // Audit Trail Tests
    #[test]
    fn test_audit_trail_recording() {
        let mut system = DecisionProvenanceSystem::new();
        let entry = AuditEntry {
            change_id: "audit-1".to_string(),
            action: "submitted".to_string(),
            actor: "user1".to_string(),
            timestamp: 0,
        };

        system.record_entry(entry);
        let entries = system.get_audit_trail("audit-1");
        assert_eq!(entries.len(), 1);
    }
}
