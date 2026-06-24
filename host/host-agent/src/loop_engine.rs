/// Agent loop engine — the core iteration through observe → understand → plan → execute → verify → reflect → persist.
///
/// Every loop operates on exactly one WorkItem at a time.
/// Supports persistence via WorkItemStore and model-assisted inference via InferenceBackend.
use crabjar_host_core::{Status, WorkItem, event_bus::EventBus};
use crabjar_host_observe::MetricsCollector;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

use crate::inference::{InferenceBackend, InferenceConfig};
use crate::work_item_store::WorkItemStore;

/// The agent loop engine.
pub struct AgentLoop {
    event_bus: Arc<EventBus>,
    metrics: MetricsCollector,
    max_iterations: u32,
    confidence_threshold: f32,
    current_work_item: Option<WorkItem>,
    iteration: u32,
    /// Optional persistent store for WorkItem (enables restart recovery).
    store: Option<Arc<WorkItemStore>>,
    /// Optional inference backend for model-assisted stages.
    inference: Option<Box<dyn InferenceBackend>>,
}

impl AgentLoop {
    pub fn new(event_bus: Arc<EventBus>, metrics: MetricsCollector) -> Self {
        Self {
            event_bus,
            metrics,
            max_iterations: 100,
            confidence_threshold: 0.85,
            current_work_item: None,
            iteration: 0,
            store: None,
            inference: None,
        }
    }

    /// Create a loop with persistence and optional inference.
    pub fn new_with_persistence(
        event_bus: Arc<EventBus>,
        metrics: MetricsCollector,
        db_path: PathBuf,
        inference_config: Option<InferenceConfig>,
    ) -> Result<Self, rusqlite::Error> {
        #[allow(clippy::arc_with_non_send_sync)]
        let store = Arc::new(WorkItemStore::open(db_path)?);
        let inference = inference_config.map(|cfg| {
            let backend: Box<dyn InferenceBackend> = crate::inference::create_backend(&cfg);
            backend
        });
        Ok(Self {
            event_bus,
            metrics,
            max_iterations: 100,
            confidence_threshold: 0.85,
            current_work_item: None,
            iteration: 0,
            store: Some(store),
            inference,
        })
    }

    /// Start a new WorkItem.
    pub fn start(&mut self, objective: impl Into<String>) {
        let work_item = WorkItem::new(objective);
        self.current_work_item = Some(work_item);
        self.iteration = 0;
        tracing::info!("agent loop started");
    }

    /// Start a new WorkItem, or resume the most recent persisted one.
    pub async fn start_with_resume(
        &mut self,
        objective: impl Into<String>,
    ) -> Result<(), crate::work_item_store::WorkItemStoreError> {
        // Try to resume a persisted work item first
        if let Some(ref store) = self.store {
            let ids = store.list_ids().await?;
            if !ids.is_empty() {
                let latest_id = ids[0];
                if let Ok(resumed) = store.load(latest_id).await
                    && !resumed.status.is_terminal()
                {
                    tracing::info!(work_item_id = ?latest_id, "resumed persisted work item");
                    self.current_work_item = Some(resumed);
                    self.iteration = 0;
                    return Ok(());
                }
            }
        }

        // No resume possible — start fresh
        self.start(objective);
        Ok(())
    }

    /// Run one iteration of the agent loop.
    pub async fn tick(&mut self) -> Result<LoopResult, LoopError> {
        // Take the work item out to avoid borrow overlap with self methods
        let mut work_item = self.current_work_item.take().ok_or(LoopError::NoWorkItem)?;

        self.iteration += 1;

        // Check max iterations
        if self.iteration > self.max_iterations {
            work_item.set_status(Status::Failed {
                reason: format!("max iterations ({}) exceeded", self.max_iterations),
            });
            let wid = work_item.id;
            self.persist_work_item(&work_item).await;
            return Ok(LoopResult::Failed {
                work_item_id: wid,
                reason: "max iterations exceeded".into(),
            });
        }

        // Check confidence threshold
        if work_item.confidence >= self.confidence_threshold {
            work_item.set_status(Status::Completed);
            let wid = work_item.id;
            self.persist_work_item(&work_item).await;
            return Ok(LoopResult::Completed { work_item_id: wid });
        }

        // Record iteration metric
        let mut labels = std::collections::HashMap::new();
        labels.insert("stage".into(), "tick".into());
        self.metrics.inc("agent_iterations", labels);

        // Run the loop stages (no borrow overlap since we took ownership)
        let result = self.run_loop_stages(&mut work_item).await?;

        // Update confidence based on results
        self.update_confidence(&mut work_item);

        let wid = work_item.id;

        // Persist work item after tick
        self.persist_work_item(&work_item).await;

        // Put the work item back
        self.current_work_item = Some(work_item);

        // Publish stage event
        let _ = self.event_bus.publish_typed(
            crabjar_host_core::event_bus::EventType::Agent {
                stage: "tick".into(),
                work_item_id: wid.to_string(),
            },
            "agent-loop",
        );

        Ok(result)
    }

    /// Persist the current work item to the store (no-op if no store configured).
    async fn persist_work_item(&self, work_item: &WorkItem) {
        if let Some(ref store) = self.store
            && let Err(e) = store.save(work_item).await
        {
            tracing::warn!(error = ?e, "failed to persist work item");
        }
    }

    /// Run all stages of the agent loop for the current WorkItem.
    async fn run_loop_stages(&mut self, work_item: &mut WorkItem) -> Result<LoopResult, LoopError> {
        // Stage 1: Observe — gather current state
        work_item.progress_to(Status::Understanding);
        self.observe(work_item).await?;

        // Stage 2: Understand — analyze observations
        work_item.progress_to(Status::Planning);
        self.understand(work_item).await?;

        // Stage 3: Plan — generate tasks
        self.plan(work_item).await?;

        // Stage 4: Execute — run tasks
        work_item.progress_to(Status::Executing { current_task: None });
        self.execute(work_item).await?;

        // Stage 5: Verify — check results
        work_item.progress_to(Status::Verifying);
        self.verify(work_item).await?;

        // Stage 6: Reflect — evaluate
        work_item.progress_to(Status::Reflecting);
        self.reflect(work_item).await?;

        // Stage 7: Persist — save state
        work_item.progress_to(Status::Pending);
        self.persist(work_item).await?;

        Ok(LoopResult::IterationComplete {
            work_item_id: work_item.id,
            confidence: work_item.confidence,
            tasks_completed: work_item
                .plan
                .iter()
                .filter(|t| {
                    matches!(
                        t.status,
                        crabjar_host_core::work_item::TaskStatus::Completed
                    )
                })
                .count(),
        })
    }

    async fn observe(&self, work_item: &mut WorkItem) -> Result<(), LoopError> {
        work_item.observe("observe", "state", "Gathering current workspace state");
        if let Some(ref inference) = self.inference {
            let prompt = format!(
                "Observe phase for work item '{}'. Current status: {:?}. Plan: {:?}. What should be observed next?",
                work_item.objective, work_item.status, work_item.plan
            );
            match inference.infer(&prompt).await {
                Ok(response) => {
                    work_item.observe("observe", "inference", response);
                }
                Err(e) => {
                    tracing::warn!(error = ?e, "inference failed during observe");
                }
            }
        }
        Ok(())
    }

    async fn understand(&self, work_item: &mut WorkItem) -> Result<(), LoopError> {
        work_item.observe("understand", "analysis", "Analyzing gathered state");
        if let Some(ref inference) = self.inference {
            let prompt = format!(
                "Understand phase for work item '{}'. Observations so far: {:?}. Analyze the problem and suggest key insights.",
                work_item.objective, work_item.observations
            );
            match inference.infer(&prompt).await {
                Ok(response) => {
                    work_item.observe("understand", "inference", response);
                }
                Err(e) => {
                    tracing::warn!(error = ?e, "inference failed during understand");
                }
            }
        }
        Ok(())
    }

    async fn plan(&self, work_item: &mut WorkItem) -> Result<(), LoopError> {
        work_item.observe("plan", "planning", "Generating execution plan");
        if let Some(ref inference) = self.inference {
            let prompt = format!(
                "Plan phase for work item '{}'. Objective: '{}'. Current observations: {:?}. Generate a concrete task plan.",
                work_item.objective, work_item.objective, work_item.observations
            );
            match inference.infer(&prompt).await {
                Ok(response) => {
                    work_item.observe("plan", "inference", response);
                }
                Err(e) => {
                    tracing::warn!(error = ?e, "inference failed during plan");
                }
            }
        }
        Ok(())
    }

    async fn execute(&self, work_item: &mut WorkItem) -> Result<(), LoopError> {
        work_item.observe("execute", "execution", "Executing planned tasks");
        if let Some(ref inference) = self.inference {
            let prompt = format!(
                "Execute phase for work item '{}'. Plan: {:?}. Current confidence: {:.2}. Which task should be prioritized?",
                work_item.objective, work_item.plan, work_item.confidence
            );
            match inference.infer(&prompt).await {
                Ok(response) => {
                    work_item.observe("execute", "inference", response);
                }
                Err(e) => {
                    tracing::warn!(error = ?e, "inference failed during execute");
                }
            }
        }
        Ok(())
    }

    async fn verify(&self, work_item: &mut WorkItem) -> Result<(), LoopError> {
        work_item.observe("verify", "verification", "Verifying execution results");
        if let Some(ref inference) = self.inference {
            let prompt = format!(
                "Verify phase for work item '{}'. Task results: {:?}. Are the results satisfactory?",
                work_item.objective,
                work_item
                    .plan
                    .iter()
                    .map(|t| (t.id, t.result.clone()))
                    .collect::<Vec<_>>()
            );
            match inference.infer(&prompt).await {
                Ok(response) => {
                    work_item.observe("verify", "inference", response);
                }
                Err(e) => {
                    tracing::warn!(error = ?e, "inference failed during verify");
                }
            }
        }
        Ok(())
    }

    async fn reflect(&self, work_item: &mut WorkItem) -> Result<(), LoopError> {
        work_item.observe(
            "reflect",
            "reflection",
            "Evaluating results and updating confidence",
        );
        if let Some(ref inference) = self.inference {
            let prompt = format!(
                "Reflect phase for work item '{}'. Confidence: {:.2}. Plan progress: {:?}. Should we continue, retry, or conclude?",
                work_item.objective, work_item.confidence, work_item.plan
            );
            match inference.infer(&prompt).await {
                Ok(response) => {
                    work_item.observe("reflect", "inference", response);
                }
                Err(e) => {
                    tracing::warn!(error = ?e, "inference failed during reflect");
                }
            }
        }
        Ok(())
    }

    async fn persist(&self, work_item: &mut WorkItem) -> Result<(), LoopError> {
        work_item.observe("persist", "storage", "Persisting WorkItem state");
        // Persistence is handled by persist_work_item() in tick()
        Ok(())
    }

    /// Update confidence based on task completion ratio.
    fn update_confidence(&self, work_item: &mut WorkItem) {
        let total = work_item.plan.len();
        if total == 0 {
            return;
        }
        let completed = work_item
            .plan
            .iter()
            .filter(|t| {
                matches!(
                    t.status,
                    crabjar_host_core::work_item::TaskStatus::Completed
                )
            })
            .count() as f32;
        let ratio = completed / total as f32;
        // Confidence = weighted average of task completion ratio
        let new_confidence = work_item.confidence * 0.7 + ratio * 0.3;
        work_item.set_confidence(new_confidence);
    }

    /// Get the current work item (if any).
    pub fn current_work_item(&self) -> Option<&WorkItem> {
        self.current_work_item.as_ref()
    }

    /// Get the current iteration count.
    pub fn iteration(&self) -> u32 {
        self.iteration
    }

    /// Set the max iterations limit.
    pub fn set_max_iterations(&mut self, max: u32) {
        self.max_iterations = max;
    }

    /// Set the confidence threshold for auto-completion.
    pub fn set_confidence_threshold(&mut self, threshold: f32) {
        self.confidence_threshold = threshold;
    }
}

/// Result of a single agent loop iteration.
#[derive(Debug, Clone, serde::Serialize)]
pub enum LoopResult {
    /// Iteration completed, continuing
    IterationComplete {
        work_item_id: Uuid,
        confidence: f32,
        tasks_completed: usize,
    },
    /// WorkItem completed (confidence threshold reached)
    Completed { work_item_id: Uuid },
    /// WorkItem failed
    Failed { work_item_id: Uuid, reason: String },
}

/// Agent loop errors.
#[derive(thiserror::Error, Debug)]
pub enum LoopError {
    #[error("no active work item")]
    NoWorkItem,
    #[error("observe stage failed: {0}")]
    ObserveFailed(String),
    #[error("understand stage failed: {0}")]
    UnderstandFailed(String),
    #[error("plan stage failed: {0}")]
    PlanFailed(String),
    #[error("execute stage failed: {0}")]
    ExecuteFailed(String),
    #[error("verify stage failed: {0}")]
    VerifyFailed(String),
    #[error("reflect stage failed: {0}")]
    ReflectFailed(String),
    #[error("persist stage failed: {0}")]
    PersistFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_loop_starts_and_ticks() {
        let bus = Arc::new(EventBus::new(16));
        let metrics = MetricsCollector::new();
        let mut loop_engine = AgentLoop::new(bus, metrics);

        loop_engine.start("Test objective");
        assert!(loop_engine.current_work_item().is_some());

        let result = loop_engine.tick().await.unwrap();
        assert!(matches!(result, LoopResult::IterationComplete { .. }));
        assert_eq!(loop_engine.iteration(), 1);
    }

    #[tokio::test]
    async fn test_loop_without_work_item() {
        let bus = Arc::new(EventBus::new(16));
        let metrics = MetricsCollector::new();
        let mut loop_engine = AgentLoop::new(bus, metrics);

        let result = loop_engine.tick().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_persistence_save_and_resume() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.into_path().join("work_items.db");

        let bus = Arc::new(EventBus::new(16));
        let metrics = MetricsCollector::new();
        let mut loop_engine =
            AgentLoop::new_with_persistence(bus, metrics, db_path.clone(), None).unwrap();

        loop_engine.start("Persistent task");
        let work_item_id = loop_engine.current_work_item().unwrap().id;
        let result = loop_engine.tick().await.unwrap();
        assert!(matches!(result, LoopResult::IterationComplete { .. }));

        // Drop the loop engine
        drop(loop_engine);

        // Create a new engine and resume
        let bus2 = Arc::new(EventBus::new(16));
        let metrics2 = MetricsCollector::new();
        let mut loop_engine2 =
            AgentLoop::new_with_persistence(bus2, metrics2, db_path, None).unwrap();

        // start_with_resume should find the persisted work item
        loop_engine2
            .start_with_resume("Fallback objective")
            .await
            .unwrap();
        let resumed = loop_engine2.current_work_item().unwrap();
        assert_eq!(resumed.id, work_item_id);
        assert_eq!(resumed.objective, "Persistent task");
    }

    #[tokio::test]
    async fn test_inference_backend_heuristic_default() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.into_path().join("work_items.db");

        let bus = Arc::new(EventBus::new(16));
        let metrics = MetricsCollector::new();
        let config = InferenceConfig::default();
        let mut loop_engine =
            AgentLoop::new_with_persistence(bus, metrics, db_path, Some(config)).unwrap();

        loop_engine.start("Heuristic inference test");
        let result = loop_engine.tick().await.unwrap();
        assert!(matches!(result, LoopResult::IterationComplete { .. }));

        // Verify heuristic inference was recorded
        let wi = loop_engine.current_work_item().unwrap();
        let has_inference = wi.observations.iter().any(|o| o.kind == "inference");
        assert!(has_inference, "expected heuristic inference observation");
    }
}
