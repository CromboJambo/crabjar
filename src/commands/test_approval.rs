//! End-to-End Integration Tests for Approval Workflow
//!
//! Verifies impact assessment, approval queue management, status updates,
//! and audit trail functionality.

#[cfg(test)]
mod tests {
    use crate::commands::impact_assessment::{assess_impact, ImpactLevel, ServiceCriticality};
    use crate::commands::approval_models::{ApprovalQueue, ApprovalRequest, ApprovalStatus, ChangeRequest, ImpactLevel as ModelImpact};
    use crate::commands::approval_service::{ApprovalQueueService, ChangeRequest as ServiceChange};

    #[test]
    fn test_impact_assessment_classification() {
        // Low impact: few users, standard service, config change
        let low = assess_impact(5, vec![ServiceCriticality::Standard], "config", false);
        assert_eq!(low.level, ImpactLevel::Low);

        // Medium impact: moderate users
        let medium = assess_impact(50, vec![ServiceCriticality::Standard], "config", false);
        assert_eq!(medium.level, ImpactLevel::Medium);

        // High impact: many users
        let high = assess_impact(200, vec![ServiceCriticality::Standard], "config", false);
        assert_eq!(high.level, ImpactLevel::High);
    }

    #[test]
    fn test_approval_queue_accepts_change_request() {
        let mut queue = ApprovalQueueService::new();

        let change = ServiceChange {
            id: "change-1".to_string(),
            title: "Fix critical bug".to_string(),
            summary: "A critical bug found in the payment module".to_string(),
            submitter_id: "alice".to_string(),
            submitted_at: 0,
            status: crate::commands::approval_service::ChangeStatus::Pending,
            affected_projects: vec!["payment".to_string()],
            required_approvals: vec!["bob".to_string()],
            min_approvals_required: 1,
            approvals_received: 0,
            rejection_reason: None,
        };

        let id = queue.add_change(change);
        assert_eq!(id, "change-1");

        let pending = queue.get_pending();
        assert_eq!(pending.len(), 1);
    }

    #[test]
    fn test_approval_queue_updates_status_to_approved() {
        let mut queue = ApprovalQueueService::new();

        let change = ServiceChange {
            id: "change-2".to_string(),
            title: "Update documentation".to_string(),
            summary: "Update user guide".to_string(),
            submitter_id: "bob".to_string(),
            submitted_at: 0,
            status: crate::commands::approval_service::ChangeStatus::Pending,
            affected_projects: vec!["docs".to_string()],
            required_approvals: vec!["charlie".to_string()],
            min_approvals_required: 1,
            approvals_received: 0,
            rejection_reason: None,
        };

        queue.add_change(change);

        // Approve the change
        let approved = queue.approve_change("change-2", "charlie");
        assert!(approved);

        // Verify status updated
        let pending = queue.get_pending();
        assert_eq!(pending.len(), 0);
    }

    #[test]
    fn test_approval_queue_updates_status_to_rejected() {
        let mut queue = ApprovalQueueService::new();

        let change = ServiceChange {
            id: "change-3".to_string(),
            title: "Remove deprecated feature".to_string(),
            summary: "Clean up old code".to_string(),
            submitter_id: "dave".to_string(),
            submitted_at: 0,
            status: crate::commands::approval_service::ChangeStatus::Pending,
            affected_projects: vec!["legacy".to_string()],
            required_approvals: vec!["eve".to_string()],
            min_approvals_required: 1,
            approvals_received: 0,
            rejection_reason: None,
        };

        queue.add_change(change);

        // Reject the change
        let rejected = queue.reject_change("change-3", "eve", "Feature still in use");
        assert!(rejected);

        // Verify status updated
        let pending = queue.get_pending();
        assert_eq!(pending.len(), 0);
    }

    #[test]
    fn test_audit_trail_records_all_decisions() {
        let mut queue = ApprovalQueueService::new();

        let change = ServiceChange {
            id: "change-4".to_string(),
            title: "Add new feature".to_string(),
            summary: "Implement new functionality".to_string(),
            submitter_id: "frank".to_string(),
            submitted_at: 0,
            status: crate::commands::approval_service::ChangeStatus::Pending,
            affected_projects: vec!["feature-x".to_string()],
            required_approvals: vec!["grace".to_string()],
            min_approvals_required: 1,
            approvals_received: 0,
            rejection_reason: None,
        };

        queue.add_change(change);

        // Check audit trail has submission entry
        let audit = queue.get_audit_trail("change-4");
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].action, "submitted");
    }
}
