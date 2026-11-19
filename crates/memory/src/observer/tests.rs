//! Tests for observer worker state.

use crate::lineage::models::LineageId;
use crate::observer::models::{
    EnvironmentId, EvidenceRef, FindingCategory, FindingRef, ObserverState,
};
use crate::observer::state_machine::{WorkerLifecycleError, WorkerLifecycleState};
use crate::waypoints::models::WaypointId;

#[test]
fn test_environment_id_display() {
    let env = EnvironmentId::new("prod-eu-west-1");
    assert_eq!(env.to_string(), "prod-eu-west-1");
}

#[test]
fn test_finding_category_display() {
    assert_eq!(
        FindingCategory::StructuralInsight.to_string(),
        "structural-insight"
    );
    assert_eq!(
        FindingCategory::OperationalConcern.to_string(),
        "operational-concern"
    );
    assert_eq!(
        FindingCategory::ImprovementIdea.to_string(),
        "improvement-idea"
    );
    assert_eq!(
        FindingCategory::AnomalyDetected.to_string(),
        "anomaly-detected"
    );
    assert_eq!(FindingCategory::KnowledgeGap.to_string(), "knowledge-gap");
}

#[test]
fn test_evidence_ref_waypoint() {
    let wp = WaypointId("wp-1".to_string());
    let ev = EvidenceRef::waypoint(wp, "deployment pipeline overview");
    assert_eq!(ev.id, "wp-1");
    assert_eq!(
        ev.description.as_deref(),
        Some("deployment pipeline overview")
    );
}

#[test]
fn test_evidence_ref_artifact() {
    let ev = EvidenceRef::artifact("artifact-42", "build log");
    assert_eq!(ev.id, "artifact-42");
    assert_eq!(ev.kind.to_string(), "artifact");
}

#[test]
fn test_finding_ref_new() {
    let finding = FindingRef::new(
        "f1",
        FindingCategory::StructuralInsight,
        "Circular dependency between guard and memory crates",
        0.85,
    );
    assert_eq!(finding.id.to_string(), "f1");
    assert_eq!(finding.category, FindingCategory::StructuralInsight);
    assert!((finding.confidence - 0.85).abs() < f32::EPSILON);
    assert!(finding.evidence.is_empty());
}

#[test]
fn test_finding_confidence_clamped() {
    let finding = FindingRef::new("f1", FindingCategory::AnomalyDetected, "test", 1.5);
    assert_eq!(finding.confidence, 1.0);

    let finding = FindingRef::new("f2", FindingCategory::AnomalyDetected, "test", -0.5);
    assert_eq!(finding.confidence, 0.0);
}

#[test]
fn test_finding_is_speculative() {
    let finding = FindingRef::new(
        "f1",
        FindingCategory::ImprovementIdea,
        "Could cache lookups",
        0.3,
    );
    assert!(finding.is_speculative(0.5));
    assert!(!finding.is_speculative(0.2));
}

#[test]
fn test_finding_with_evidence() {
    let finding = FindingRef::new("f1", FindingCategory::OperationalConcern, "test", 0.7)
        .with_evidence(EvidenceRef::artifact("log-1", "error log"));
    assert_eq!(finding.evidence.len(), 1);
}

#[test]
fn test_observer_state_new() {
    let env = EnvironmentId::new("test-env");
    let state = ObserverState::new("worker-42", env);
    assert_eq!(state.worker_id, "worker-42");
    assert_eq!(state.environment.to_string(), "test-env");
    assert!(state.focus_areas.is_empty());
    assert!(state.questions.is_empty());
    assert_eq!(state.max_traversal_depth, 3);
    assert_eq!(state.max_findings_before_review, 5);
}

#[test]
fn test_observer_state_with_focus_and_question() {
    let env = EnvironmentId::new("prod");
    let wp = WaypointId("wp-deploy".to_string());
    let state = ObserverState::new("w1", env)
        .with_focus(wp)
        .with_question("How does the deployment pipeline handle rollbacks?");

    assert_eq!(state.focus_areas.len(), 1);
    assert_eq!(state.questions.len(), 1);
}

#[test]
fn test_observer_add_finding_and_needs_review() {
    let env = EnvironmentId::new("prod");
    let mut state = ObserverState::new("w1", env);
    state.max_findings_before_review = 2;

    assert!(!state.needs_review());

    let needs = state.add_finding(FindingRef::new(
        "f1",
        FindingCategory::AnomalyDetected,
        "CPU spike detected",
        0.9,
    ));
    assert!(!needs);
    assert!(!state.needs_review());

    let needs = state.add_finding(FindingRef::new(
        "f2",
        FindingCategory::AnomalyDetected,
        "Memory leak suspected",
        0.75,
    ));
    assert!(needs);
    assert!(state.needs_review());
}

#[test]
fn test_observer_duration_and_expiry() {
    let env = EnvironmentId::new("prod");
    let now = 1_000_000;
    let mut state = ObserverState::new("w1", env);
    state.started_at = now - 3600; // 1 hour ago

    assert_eq!(state.duration_secs(now), 3600);
    assert!(!state.is_expired(now, 7200));
    assert!(state.is_expired(now, 1800));
}

#[test]
fn test_lifecycle_observer_to_working() {
    let state = WorkerLifecycleState::Observer;
    let working = state.assign_task("task-42").unwrap();
    assert_eq!(
        working,
        WorkerLifecycleState::Working {
            task_id: "task-42".to_string()
        }
    );
    assert!(working.is_working());
}

#[test]
fn test_lifecycle_working_to_observer() {
    let state = WorkerLifecycleState::Working {
        task_id: "task-42".to_string(),
    };
    let observer = state.return_to_observer().unwrap();
    assert_eq!(observer, WorkerLifecycleState::Observer);
    assert!(observer.is_observer());
}

#[test]
fn test_lifecycle_destroy_from_any() {
    for state in [
        WorkerLifecycleState::Observer,
        WorkerLifecycleState::Working {
            task_id: "t".to_string(),
        },
    ] {
        let destroyed = state.destroy().unwrap();
        assert!(destroyed.is_destroyed());
    }

    // Destroy from destroyed is invalid
    let err = WorkerLifecycleState::Destroyed.destroy().unwrap_err();
    assert_eq!(
        err,
        WorkerLifecycleError::InvalidTransition {
            from: WorkerLifecycleState::Destroyed,
            to: WorkerLifecycleState::Destroyed,
        }
    );
}

#[test]
fn test_lifecycle_invalid_transitions() {
    // Assign task while already working
    let err = WorkerLifecycleState::Working {
        task_id: "t1".to_string(),
    }
    .assign_task("t2")
    .unwrap_err();
    assert!(matches!(
        err,
        WorkerLifecycleError::InvalidTransition { .. }
    ));

    // Return to observer while already observing
    let err = WorkerLifecycleState::Observer
        .return_to_observer()
        .unwrap_err();
    assert!(matches!(
        err,
        WorkerLifecycleError::InvalidTransition { .. }
    ));
}

#[test]
fn test_lifecycle_current_task_id() {
    assert_eq!(
        WorkerLifecycleState::Working {
            task_id: "t42".to_string()
        }
        .current_task_id(),
        Some("t42")
    );
    assert_eq!(WorkerLifecycleState::Observer.current_task_id(), None);
    assert_eq!(WorkerLifecycleState::Destroyed.current_task_id(), None);
}

#[test]
fn test_lifecycle_display() {
    assert_eq!(WorkerLifecycleState::Observer.to_string(), "observer");
    assert_eq!(
        WorkerLifecycleState::Working {
            task_id: "t".to_string()
        }
        .to_string(),
        "working(t)"
    );
    assert_eq!(WorkerLifecycleState::Destroyed.to_string(), "destroyed");
}

#[test]
fn test_observer_state_with_lineage_observations() {
    let env = EnvironmentId::new("prod");
    let mut state = ObserverState::new("w1", env);
    state.observations.push(LineageId("obs-1".to_string()));
    state.observations.push(LineageId("obs-2".to_string()));
    assert_eq!(state.observations.len(), 2);
}
