//! Worker lifecycle state machine — Observer / Working / Destroyed.

use std::fmt;

/// Error that can occur during a worker lifecycle transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerLifecycleError {
    /// Transition is not valid from the current state.
    InvalidTransition {
        from: WorkerLifecycleState,
        to: WorkerLifecycleState,
    },
    /// Worker has no capacity remaining.
    NoCapacity,
    /// Observer duration exceeded maximum allowed.
    DurationExceeded { elapsed_secs: i64, max_secs: i64 },
}

impl fmt::Display for WorkerLifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkerLifecycleError::InvalidTransition { from, to } => {
                write!(f, "invalid transition from {} to {}", from, to)
            }
            WorkerLifecycleError::NoCapacity => write!(f, "worker has no capacity remaining"),
            WorkerLifecycleError::DurationExceeded {
                elapsed_secs,
                max_secs,
            } => {
                write!(
                    f,
                    "observer duration exceeded: {}s > {}s",
                    elapsed_secs, max_secs
                )
            }
        }
    }
}

impl std::error::Error for WorkerLifecycleError {}

/// Worker lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WorkerLifecycleState {
    /// Worker is observing the environment — building situational awareness.
    Observer,
    /// Worker is actively working on an assigned task.
    Working { task_id: String },
    /// Worker has been destroyed or recycled.
    Destroyed,
}

impl fmt::Display for WorkerLifecycleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkerLifecycleState::Observer => write!(f, "observer"),
            WorkerLifecycleState::Working { task_id } => write!(f, "working({})", task_id),
            WorkerLifecycleState::Destroyed => write!(f, "destroyed"),
        }
    }
}

impl WorkerLifecycleState {
    /// Transition from Observer to Working when a task is assigned.
    pub fn assign_task(self, task_id: impl Into<String>) -> Result<Self, WorkerLifecycleError> {
        match self {
            WorkerLifecycleState::Observer => Ok(WorkerLifecycleState::Working {
                task_id: task_id.into(),
            }),
            WorkerLifecycleState::Working { .. } => Err(WorkerLifecycleError::InvalidTransition {
                from: self,
                to: WorkerLifecycleState::Working {
                    task_id: task_id.into(),
                },
            }),
            WorkerLifecycleState::Destroyed => Err(WorkerLifecycleError::InvalidTransition {
                from: self,
                to: WorkerLifecycleState::Working {
                    task_id: task_id.into(),
                },
            }),
        }
    }

    /// Transition from Working back to Observer (task complete, capacity remains).
    pub fn return_to_observer(self) -> Result<Self, WorkerLifecycleError> {
        match self {
            WorkerLifecycleState::Working { .. } => Ok(WorkerLifecycleState::Observer),
            WorkerLifecycleState::Observer => Err(WorkerLifecycleError::InvalidTransition {
                from: self,
                to: WorkerLifecycleState::Observer,
            }),
            WorkerLifecycleState::Destroyed => Err(WorkerLifecycleError::InvalidTransition {
                from: self,
                to: WorkerLifecycleState::Observer,
            }),
        }
    }

    /// Transition to Destroyed (recycled or error).
    pub fn destroy(self) -> Result<Self, WorkerLifecycleError> {
        match self {
            WorkerLifecycleState::Destroyed => Err(WorkerLifecycleError::InvalidTransition {
                from: self,
                to: WorkerLifecycleState::Destroyed,
            }),
            _ => Ok(WorkerLifecycleState::Destroyed),
        }
    }

    /// Check if the worker is in observer mode.
    pub fn is_observer(&self) -> bool {
        matches!(self, WorkerLifecycleState::Observer)
    }

    /// Check if the worker is actively working.
    pub fn is_working(&self) -> bool {
        matches!(self, WorkerLifecycleState::Working { .. })
    }

    /// Check if the worker has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        matches!(self, WorkerLifecycleState::Destroyed)
    }

    /// Get the task ID if the worker is working.
    pub fn current_task_id(&self) -> Option<&str> {
        match self {
            WorkerLifecycleState::Working { task_id } => Some(task_id),
            _ => None,
        }
    }
}
