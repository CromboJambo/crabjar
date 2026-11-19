#[cfg(test)]
mod tests {
    use super::*;
    use crate::trust_types::TrustScore;

    #[test]
    fn action_status_display_pending() {
        assert_eq!(format!("{}", ActionStatus::Pending), "pending");
    }

    #[test]
    fn action_status_display_trust_approved() {
        assert_eq!(format!("{}", ActionStatus::TrustApproved), "trust-approved");
    }

    #[test]
    fn action_status_display_denied() {
        assert_eq!(format!("{}", ActionStatus::Denied), "denied");
    }

    #[test]
    fn action_status_display_executed() {
        assert_eq!(format!("{}", ActionStatus::Executed), "executed");
    }

    #[test]
    fn action_status_display_interrupted() {
        assert_eq!(format!("{}", ActionStatus::Interrupted), "interrupted");
    }

    #[test]
    fn outcome_status_display_executed() {
        assert_eq!(format!("{}", OutcomeStatus::Executed), "executed");
    }

    #[test]
    fn outcome_status_display_executed_trust_update_failed() {
        assert_eq!(
            format!("{}", OutcomeStatus::ExecutedTrustUpdateFailed),
            "executed-trust-update-failed"
        );
    }

    #[test]
    fn action_status_equality() {
        assert_eq!(ActionStatus::Pending, ActionStatus::Pending);
        assert_ne!(ActionStatus::Pending, ActionStatus::Denied);
    }

    #[test]
    fn outcome_status_equality() {
        assert_eq!(OutcomeStatus::Executed, OutcomeStatus::Executed);
        assert_ne!(
            OutcomeStatus::Executed,
            OutcomeStatus::ExecutedTrustUpdateFailed
        );
    }

    #[test]
    fn action_status_helpers() {
        assert!(ActionStatus::Pending.is_pending());
        assert!(!ActionStatus::Executed.is_pending());

        assert!(!ActionStatus::Pending.is_executed());
        assert!(ActionStatus::Executed.is_executed());

        assert!(!ActionStatus::Pending.is_denied());
        assert!(ActionStatus::Denied.is_denied());
    }

    #[test]
    fn action_request_new() {
        let request =
            ActionRequest::new("test-1", "execute", "echo hello", 2, TrustScore::new(0.7));
        assert_eq!(request.id, "test-1");
        assert_eq!(request.action_type, "execute");
        assert_eq!(request.trust_layer, 2);
        assert_eq!(request.status, ActionStatus::Pending);
        assert!(request.source_event_id.is_none());
        assert!(request.source_node_id.is_none());
    }

    #[test]
    fn action_request_with_source() {
        let request = ActionRequest::new("test-2", "execute", "echo", 2, TrustScore::new(0.5))
            .with_source("event-123");
        assert_eq!(request.source_event_id, Some("event-123".to_string()));
    }

    #[test]
    fn action_request_with_node() {
        let request = ActionRequest::new("test-3", "execute", "echo", 2, TrustScore::new(0.5))
            .with_node("node-456");
        assert_eq!(request.source_node_id, Some("node-456".to_string()));
    }

    #[test]
    fn action_request_mark_approved() {
        let request = ActionRequest::new("test-4", "execute", "echo", 2, TrustScore::new(0.5))
            .mark_approved();
        assert_eq!(request.status, ActionStatus::TrustApproved);
    }

    #[test]
    fn action_request_mark_denied() {
        let request =
            ActionRequest::new("test-5", "execute", "echo", 2, TrustScore::new(0.5)).mark_denied();
        assert_eq!(request.status, ActionStatus::Denied);
    }

    #[test]
    fn action_request_mark_executed() {
        let request = ActionRequest::new("test-6", "execute", "echo", 2, TrustScore::new(0.5))
            .mark_executed();
        assert_eq!(request.status, ActionStatus::Executed);
        assert!(request.resolved_at.is_some());
    }

    #[test]
    fn action_request_mark_interrupted() {
        let request = ActionRequest::new("test-7", "execute", "echo", 2, TrustScore::new(0.5))
            .mark_interrupted();
        assert_eq!(request.status, ActionStatus::Interrupted);
        assert!(request.resolved_at.is_some());
    }

    #[test]
    fn action_outcome_new() {
        let outcome = ActionOutcome::new("out-1", "req-1", true, 0.1);
        assert_eq!(outcome.id, "out-1");
        assert_eq!(outcome.action_id, "req-1");
        assert!(outcome.success);
        assert_eq!(outcome.confidence_delta, 0.1);
        assert!(outcome.exit_code.is_none());
        assert!(outcome.output_hash.is_none());
    }

    #[test]
    fn action_outcome_with_all_fields() {
        let outcome = ActionOutcome::new("out-2", "req-2", false, -0.05)
            .with_exit_code(1)
            .with_output_hash("abc123")
            .with_residual("residual data")
            .with_skill_residue("skill residue");
        assert_eq!(outcome.exit_code, Some(1));
        assert_eq!(outcome.output_hash, Some("abc123".to_string()));
        assert_eq!(outcome.residual, Some("residual data".to_string()));
        assert_eq!(outcome.skill_residue, Some("skill residue".to_string()));
    }

    // ─── Typestate tests ─────────────────────────────────────────────────────

    #[test]
    fn guarded_action_new_requires_pending() {
        let request = ActionRequest::new("gs-1", "execute", "echo", 2, TrustScore::new(0.5));
        let action: GuardedAction<Pending> = GuardedAction::new(request);
        assert_eq!(action.as_ref().status, ActionStatus::Pending);
    }

    #[test]
    #[should_panic(expected = "GuardedAction::new requires a Pending action")]
    fn guarded_action_new_rejects_non_pending() {
        let request =
            ActionRequest::new("gs-2", "execute", "echo", 2, TrustScore::new(0.5)).mark_denied();
        let _action: GuardedAction<Pending> = GuardedAction::new(request);
    }

    #[test]
    fn guarded_action_approve_transitions_to_trust_approved() {
        let request = ActionRequest::new("gs-3", "execute", "echo", 2, TrustScore::new(0.8));
        let action: GuardedAction<Pending> = GuardedAction::new(request);
        let approved = action.approve();
        assert_eq!(approved.as_ref().status, ActionStatus::TrustApproved);
    }

    #[test]
    fn guarded_action_deny_transitions_to_denied() {
        let request = ActionRequest::new("gs-4", "execute", "echo", 2, TrustScore::new(0.3));
        let action: GuardedAction<Pending> = GuardedAction::new(request);
        let denied = action.deny();
        assert_eq!(denied.as_ref().status, ActionStatus::Denied);
        assert!(denied.as_ref().resolved_at.is_some());
    }

    #[test]
    fn guarded_action_execute_transitions_to_executed() {
        let request = ActionRequest::new("gs-5", "execute", "echo", 2, TrustScore::new(0.9));
        let action: GuardedAction<Pending> = GuardedAction::new(request);
        let executed = action.approve().execute();
        assert_eq!(executed.as_ref().status, ActionStatus::Executed);
        assert!(executed.as_ref().resolved_at.is_some());
    }

    #[test]
    fn guarded_action_interrupt_transitions_to_interrupted() {
        let request = ActionRequest::new("gs-6", "execute", "echo", 2, TrustScore::new(0.9));
        let action: GuardedAction<Pending> = GuardedAction::new(request);
        let interrupted = action.approve().interrupt();
        assert_eq!(interrupted.as_ref().status, ActionStatus::Interrupted);
        assert!(interrupted.as_ref().resolved_at.is_some());
    }

    #[test]
    fn guarded_action_into_inner_preserves_data() {
        let request = ActionRequest::new("gs-7", "execute", "echo hi", 3, TrustScore::new(0.7))
            .with_source("event-999");
        let action: GuardedAction<Pending> = GuardedAction::new(request);
        let plain = action.approve().execute().into_inner();
        assert_eq!(plain.id, "gs-7");
        assert_eq!(plain.action_type, "execute");
        assert_eq!(plain.payload, "echo hi");
        assert_eq!(plain.trust_layer, 3);
        assert_eq!(plain.status, ActionStatus::Executed);
        assert_eq!(plain.source_event_id, Some("event-999".to_string()));
    }

    #[test]
    fn guarded_action_as_ref_does_not_consume() {
        let request = ActionRequest::new("gs-8", "execute", "echo", 2, TrustScore::new(0.5));
        let action: GuardedAction<Pending> = GuardedAction::new(request);
        let ref_data = action.as_ref();
        assert_eq!(ref_data.id, "gs-8");
        // action is still usable after as_ref()
        let approved = action.approve();
        assert_eq!(approved.as_ref().status, ActionStatus::TrustApproved);
    }

    #[test]
    fn guarded_action_full_lifecycle() {
        // Pending → TrustApproved → Executed
        let request = ActionRequest::new(
            "gs-9",
            "execute",
            "rm -rf /tmp/test",
            4,
            TrustScore::new(0.95),
        );
        let pending: GuardedAction<Pending> = GuardedAction::new(request);
        assert_eq!(pending.as_ref().status, ActionStatus::Pending);

        let approved = pending.approve();
        assert_eq!(approved.as_ref().status, ActionStatus::TrustApproved);

        let executed = approved.execute();
        assert_eq!(executed.as_ref().status, ActionStatus::Executed);
        assert!(executed.as_ref().resolved_at.is_some());

        // Terminal state: can only inspect or unwrap
        let final_request = executed.into_inner();
        assert_eq!(final_request.status, ActionStatus::Executed);
    }

    #[test]
    fn guarded_action_denied_is_terminal() {
        let request =
            ActionRequest::new("gs-10", "execute", "dangerous cmd", 1, TrustScore::new(0.2));
        let pending: GuardedAction<Pending> = GuardedAction::new(request);
        let denied = pending.deny();
        assert_eq!(denied.as_ref().status, ActionStatus::Denied);

        // Terminal: no approve/execute/interrupt methods available (compile-time enforced)
        let final_request = denied.into_inner();
        assert_eq!(final_request.status, ActionStatus::Denied);
    }

    #[test]
    fn guarded_action_preserves_gate_result() {
        let mut request = ActionRequest::new("gs-11", "execute", "echo", 2, TrustScore::new(0.8));
        request.gate_result = Some("approved by policy engine".to_string());
        let action: GuardedAction<Pending> = GuardedAction::new(request);
        let executed = action.approve().execute();
        assert_eq!(
            executed.as_ref().gate_result,
            Some("approved by policy engine".to_string())
        );
    }
}

