//! Unit tests for loss accounting.

use crate::loss_accounting::{
    AccessCost, ApproverRef, ArtifactType, CompressionAuditEntry, CompressionAuditLog,
    CompressionPolicy, CompressionStrategy, Confidence, FidelityProfile, LossEstimate, LossProfile,
    LossProfileId, MeasurementBasis, Recoverability, SourceRef, TaskType, TruncationSide,
};

#[test]
fn test_loss_profile_id_display() {
    let id = LossProfileId::new("lp-001");
    assert_eq!(id.to_string(), "lp-001");
}

#[test]
fn test_source_ref_new() {
    let src =
        SourceRef::new("ctx-123", "worker-context").with_description("Worker context for task 42");
    assert_eq!(src.id, "ctx-123");
    assert_eq!(src.content_class, "worker-context");
    assert_eq!(
        src.description.as_deref(),
        Some("Worker context for task 42")
    );
}

#[test]
fn test_compression_strategy_display() {
    assert_eq!(
        CompressionStrategy::Summarization {
            model: "llama-3".into()
        }
        .to_string(),
        "summarize(llama-3)"
    );
    assert_eq!(
        CompressionStrategy::Truncation {
            from: TruncationSide::Tail
        }
        .to_string(),
        "truncate(Tail)"
    );
    assert_eq!(
        CompressionStrategy::Sampling { rate: 0.5 }.to_string(),
        "sample(0.50)"
    );
    assert_eq!(
        CompressionStrategy::Abstraction { level: 2 }.to_string(),
        "abstract(l2)"
    );
    assert_eq!(
        CompressionStrategy::Aggregation {
            group_by: "topic".into()
        }
        .to_string(),
        "aggregate(topic)"
    );
}

#[test]
fn test_loss_estimate_display() {
    assert_eq!(LossEstimate::None.to_string(), "none");
    let bounded = LossEstimate::Bounded {
        max_items: 5,
        description: "tool calls".into(),
    };
    assert_eq!(bounded.to_string(), "bounded(5 items): tool calls");
    let estimated = LossEstimate::Estimated {
        probability: 0.15,
        description: "details".into(),
    };
    assert_eq!(estimated.to_string(), "estimated(15%): details");
    let unknown = LossEstimate::Unknown {
        reason: "no baseline".into(),
    };
    assert_eq!(unknown.to_string(), "unknown: no baseline");
}

#[test]
fn test_confidence_display() {
    assert_eq!(Confidence::Verified.to_string(), "verified");
    assert_eq!(
        Confidence::High {
            basis: "measured".into()
        }
        .to_string(),
        "high(measured)"
    );
    assert_eq!(Confidence::Unknown.to_string(), "unknown");
}

#[test]
fn test_access_cost_display() {
    assert_eq!(AccessCost::Free.to_string(), "free");
    assert_eq!(AccessCost::Low { seconds: 5 }.to_string(), "~5s");
    assert_eq!(AccessCost::Prohibitive.to_string(), "prohibitive");
}

#[test]
fn test_recoverability_display() {
    assert_eq!(Recoverability::Lossless.to_string(), "lossless");
    assert_eq!(
        Recoverability::Bounded { max_depth: 2 }.to_string(),
        "bounded(depth=2)"
    );
    assert_eq!(Recoverability::ReferenceOnly.to_string(), "reference-only");
}

#[test]
fn test_loss_profile_builder() {
    let profile = LossProfile::new("lp-001", SourceRef::new("ctx-1", "context"), 1000)
        .with_strategy(CompressionStrategy::Truncation {
            from: TruncationSide::Tail,
        })
        .with_target_size(300)
        .with_final_size(250)
        .with_loss_estimate(LossEstimate::Bounded {
            max_items: 10,
            description: "tool calls".into(),
        })
        .with_confidence(Confidence::High {
            basis: "measured on similar tasks".into(),
        });

    assert_eq!(profile.id.to_string(), "lp-001");
    assert_eq!(profile.original_size, 1000);
    assert_eq!(profile.final_size, 250);
    assert!((profile.reduction_ratio - 4.0).abs() < 0.01);
    assert!(!profile.is_lossless());
}

#[test]
fn test_loss_profile_is_lossless() {
    let profile = LossProfile::new("lp-002", SourceRef::new("ctx-2", "context"), 500)
        .with_final_size(500)
        .with_loss_estimate(LossEstimate::None)
        .with_recovery(
            crate::loss_accounting::RecoveryRef::new("src-1", "/data/original")
                .with_freshness_verified(),
            Recoverability::Lossless,
        );

    assert!(profile.is_lossless());
    assert!(profile.is_recoverable());
}

#[test]
fn test_compression_policy_for_artifact() {
    let policy = CompressionPolicy::for_artifact(ArtifactType::RawObservation);
    assert_eq!(policy.max_reduction_depth, 0);
    assert!(!policy.allows_depth(1));
    assert!(policy.allows_depth(0));

    let summary_policy = CompressionPolicy::for_artifact(ArtifactType::ContextSummary);
    assert_eq!(summary_policy.max_reduction_depth, 3);
    assert!(summary_policy.allows_depth(3));
    assert!(!summary_policy.allows_depth(4));
}

#[test]
fn test_confidence_meets_requirement() {
    use crate::loss_accounting::ConfidenceLevel;

    let verified = Confidence::Verified;
    let high = Confidence::High {
        basis: "test".into(),
    };
    let medium = Confidence::Medium {
        basis: "test".into(),
    };
    let unknown = Confidence::Unknown;

    assert!(crate::loss_accounting::confidence_meets_requirement(
        &verified,
        &ConfidenceLevel::VerifiedOnly
    ));
    assert!(!crate::loss_accounting::confidence_meets_requirement(
        &high,
        &ConfidenceLevel::VerifiedOnly
    ));
    assert!(crate::loss_accounting::confidence_meets_requirement(
        &high,
        &ConfidenceLevel::HighOrBetter
    ));
    assert!(crate::loss_accounting::confidence_meets_requirement(
        &medium,
        &ConfidenceLevel::MediumOrBetter
    ));
    assert!(crate::loss_accounting::confidence_meets_requirement(
        &unknown,
        &ConfidenceLevel::Any
    ));
}

#[test]
fn test_fidelity_profile() {
    let profile = FidelityProfile::new(
        TaskType::Debugging,
        3,
        0.75,
        MeasurementBasis::Empirical { samples: 50 },
    );
    assert!(profile.meets_threshold(0.7));
    assert!(!profile.meets_threshold(0.8));
}

#[test]
fn test_task_type_display() {
    assert_eq!(TaskType::Debugging.to_string(), "debugging");
    assert_eq!(TaskType::other("custom").to_string(), "custom");
}

#[test]
fn test_compression_audit_log() {
    let log = CompressionAuditLog::new();
    assert!(log.is_empty());

    let entry = CompressionAuditEntry::new(
        "worker-1",
        SourceRef::new("ctx-1", "context"),
        LossProfileId::new("lp-001"),
        "Context exceeded token budget",
    );
    log.record(entry);

    assert_eq!(log.len(), 1);
    let worker_entries = log.entries_for_worker("worker-1");
    assert_eq!(worker_entries.len(), 1);
    let source_entries = log.entries_for_source("ctx-1");
    assert_eq!(source_entries.len(), 1);

    // Entry with approval
    let approved_entry = CompressionAuditEntry::new(
        "worker-2",
        SourceRef::new("ctx-2", "context"),
        LossProfileId::new("lp-002"),
        "Deep compression for archival",
    )
    .with_approval(ApproverRef::new("supervisor-1", "fleet-supervisor"));

    log.record(approved_entry);
    assert_eq!(log.len(), 2);
    let all = log.all_entries();
    assert_eq!(all[1].approved_by.as_ref().unwrap().id, "supervisor-1");
}

#[test]
fn test_artifact_type_display() {
    assert_eq!(ArtifactType::RawObservation.to_string(), "raw-observation");
    assert_eq!(
        ArtifactType::InstitutionalMemory.to_string(),
        "institutional-memory"
    );
}
