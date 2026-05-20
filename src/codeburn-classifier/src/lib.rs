use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClassifierError {
    #[error("classification failed: {0}")]
    Failed(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskCategory {
    Test,
    Fix,
    Refactor,
    Design,
    Documentation,
    Debugging,
    Architecture,
    Integration,
    Deployment,
    Review,
    Edit,
    Research,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskClassifier {
    pub rules: Vec<(String, TaskCategory)>,
}

impl TaskClassifier {
    pub fn new() -> Self {
        Self {
            rules: vec![
                ("test".to_string(), TaskCategory::Test),
                ("fix".to_string(), TaskCategory::Fix),
                ("refactor".to_string(), TaskCategory::Refactor),
                ("design".to_string(), TaskCategory::Design),
                ("doc".to_string(), TaskCategory::Documentation),
                ("debug".to_string(), TaskCategory::Debugging),
                ("arch".to_string(), TaskCategory::Architecture),
                ("integ".to_string(), TaskCategory::Integration),
                ("deploy".to_string(), TaskCategory::Deployment),
                ("review".to_string(), TaskCategory::Review),
                ("edit".to_string(), TaskCategory::Edit),
                ("research".to_string(), TaskCategory::Research),
            ],
        }
    }

    pub fn default() -> Self {
        Self::new()
    }
}
