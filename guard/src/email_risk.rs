//! Risk assessment for email actions.
//!
//! Email operations are categorized by their reversibility and impact:
//! - Read/list/search: Low risk (idempotent, no state change)
//! - Draft/label: Medium risk (state change, easily reversible)
//! - Send: High risk (external communication, not truly reversible)
//! - Delete/archive: Medium risk (reversible via trash or sync)

/// Email action types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailAction {
    Read,
    List,
    Search,
    Send,
    Draft,
    Label,
    Move,
    Delete,
    Archive,
}

/// Risk level for an email action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailRisk {
    Low,
    Medium,
    High,
}

impl EmailRisk {
    /// Returns `true` if this risk level requires human review before execution.
    pub fn requires_review(&self) -> bool {
        matches!(self, EmailRisk::Medium | EmailRisk::High)
    }

    /// Returns `true` if this risk level blocks auto-execution entirely.
    pub fn blocks_execution(&self) -> bool {
        matches!(self, EmailRisk::High)
    }
}

/// Assess the risk of an email action.
pub fn assess_email_action(action: EmailAction) -> EmailRisk {
    match action {
        // Read-only operations are always safe.
        EmailAction::Read | EmailAction::List | EmailAction::Search => EmailRisk::Low,
        // State changes that are easily reversible.
        EmailAction::Draft | EmailAction::Label | EmailAction::Archive => EmailRisk::Medium,
        // External communication or potentially destructive operations.
        EmailAction::Send | EmailAction::Delete | EmailAction::Move => EmailRisk::High,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_operations_are_low_risk() {
        assert_eq!(assess_email_action(EmailAction::Read), EmailRisk::Low);
        assert_eq!(assess_email_action(EmailAction::List), EmailRisk::Low);
        assert_eq!(assess_email_action(EmailAction::Search), EmailRisk::Low);
    }

    #[test]
    fn reversible_state_changes_are_medium_risk() {
        assert_eq!(assess_email_action(EmailAction::Draft), EmailRisk::Medium);
        assert_eq!(assess_email_action(EmailAction::Label), EmailRisk::Medium);
        assert_eq!(assess_email_action(EmailAction::Archive), EmailRisk::Medium);
    }

    #[test]
    fn external_communication_is_high_risk() {
        assert_eq!(assess_email_action(EmailAction::Send), EmailRisk::High);
    }

    #[test]
    fn potentially_destructive_operations_are_high_risk() {
        assert_eq!(assess_email_action(EmailAction::Delete), EmailRisk::High);
        assert_eq!(assess_email_action(EmailAction::Move), EmailRisk::High);
    }

    #[test]
    fn high_risk_blocks_execution() {
        assert!(EmailRisk::High.blocks_execution());
        assert!(!EmailRisk::Low.blocks_execution());
    }

    #[test]
    fn medium_and_high_require_review() {
        assert!(EmailRisk::Medium.requires_review());
        assert!(EmailRisk::High.requires_review());
        assert!(!EmailRisk::Low.requires_review());
    }
}