/// WorkItem — the first-class unit of agent work.
///
/// Every agent loop operates on exactly one WorkItem at a time.
/// No hidden state, everything inspectable, replayable, checkpointable.
use uuid::Uuid;
use serde::{Serialize, Deserialize};

/// Status of a WorkItem through its lifecycle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Status {
    /// Just created, not yet analyzed
    Pending,
    /// Being analyzed — understanding the problem
    Understanding,
    /// Plan has been generated
    Planning,
    /// Tasks are being executed
    Executing { current_task: Option<usize> },
    /// Verification in progress
    Verifying,
    /// Reflection phase — evaluating results
    Reflecting,
    /// Completed successfully
    Completed,
    /// Failed — may be retried
    Failed { reason: String },
    /// Paused — waiting on user input or external event
    Paused { reason: String },
}

impl Status {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Status::Completed | Status::Failed { .. })
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Status::Pending | Status::Understanding | Status::Planning | Status::Executing { .. } | Status::Verifying | Status::Reflecting)
    }
}

/// A hypothesis the agent is testing during a WorkItem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    pub id: Uuid,
    pub statement: String,
    pub confidence: f32,
    pub evidence: Vec<String>,
}

/// An observation collected during a WorkItem loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub id: Uuid,
    pub stage: String,
    pub kind: String,
    pub details: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// A single task within a WorkItem's plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: usize,
    pub description: String,
    pub status: TaskStatus,
    pub result: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed { reason: String },
}

/// The WorkItem — the agent's unit of work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItem {
    pub id: Uuid,
    pub objective: String,
    pub status: Status,
    pub observations: Vec<Observation>,
    pub hypothesis: Option<Hypothesis>,
    pub plan: Vec<Task>,
    pub artifacts: Vec<String>,
    pub confidence: f32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl WorkItem {
    pub fn new(objective: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            objective: objective.into(),
            status: Status::Pending,
            observations: Vec::new(),
            hypothesis: None,
            plan: Vec::new(),
            artifacts: Vec::new(),
            confidence: 0.0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Add an observation to this WorkItem.
    pub fn observe(&mut self, stage: impl Into<String>, kind: impl Into<String>, details: impl Into<String>) {
        self.observations.push(Observation {
            id: Uuid::new_v4(),
            stage: stage.into(),
            kind: kind.into(),
            details: details.into(),
            timestamp: chrono::Utc::now(),
        });
        self.updated_at = chrono::Utc::now();
    }

    /// Set the hypothesis being tested.
    pub fn set_hypothesis(&mut self, statement: impl Into<String>, confidence: f32) {
        self.hypothesis = Some(Hypothesis {
            id: Uuid::new_v4(),
            statement: statement.into(),
            confidence,
            evidence: Vec::new(),
        });
        self.updated_at = chrono::Utc::now();
    }

    /// Add a task to the plan.
    pub fn add_task(&mut self, description: impl Into<String>) -> usize {
        let id = self.plan.len();
        self.plan.push(Task {
            id,
            description: description.into(),
            status: TaskStatus::Pending,
            result: None,
        });
        self.updated_at = chrono::Utc::now();
        id
    }

    /// Update a task's status.
    pub fn update_task(&mut self, task_id: usize, status: TaskStatus, result: Option<String>) {
        if let Some(task) = self.plan.get_mut(task_id) {
            task.status = status;
            task.result = result;
        }
        self.updated_at = chrono::Utc::now();
    }

    /// Add an artifact path.
    pub fn add_artifact(&mut self, artifact: impl Into<String>) {
        self.artifacts.push(artifact.into());
        self.updated_at = chrono::Utc::now();
    }

    /// Update the overall confidence score.
    pub fn set_confidence(&mut self, confidence: f32) {
        self.confidence = confidence.clamp(0.0, 1.0);
        self.updated_at = chrono::Utc::now();
    }

    /// Update the status.
    pub fn set_status(&mut self, status: Status) {
        self.status = status;
        self.updated_at = chrono::Utc::now();
    }

    /// Progress through the agent loop stages.
    pub fn progress_to(&mut self, next: Status) {
        self.set_status(next);
    }

    /// Serialize to JSON (for SQLite storage / API exposure).
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workitem_lifecycle() {
        let mut wi = WorkItem::new("Port notification subsystem");
        assert_eq!(wi.status, Status::Pending);
        assert_eq!(wi.confidence, 0.0);
        assert_eq!(wi.plan.len(), 0);

        wi.add_task("Find notification code");
        wi.add_task("Locate IPC");
        assert_eq!(wi.plan.len(), 2);

        wi.update_task(0, TaskStatus::Completed, Some("Found in notify.rs".into()));
        let task = &wi.plan[0];
        assert_eq!(task.status, TaskStatus::Completed);

        wi.set_confidence(0.75);
        assert!((wi.confidence - 0.75).abs() < f32::EPSILON);

        wi.progress_to(Status::Executing { current_task: Some(1) });
        assert!(matches!(wi.status, Status::Executing { .. }));
    }

    #[test]
    fn test_status_terminal() {
        assert!(Status::Completed.is_terminal());
        assert!(Status::Failed { reason: "test".into() }.is_terminal());
        assert!(!Status::Pending.is_terminal());
        assert!(!Status::Executing { current_task: None }.is_terminal());
    }

    #[test]
    fn test_status_active() {
        assert!(Status::Pending.is_active());
        assert!(Status::Executing { current_task: None }.is_active());
        assert!(!Status::Completed.is_active());
        assert!(!Status::Failed { reason: "x".into() }.is_active());
    }
}
