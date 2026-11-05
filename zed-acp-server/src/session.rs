use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpSession {
    pub cwd: String,
    pub session_id: String,
    pub initialized: bool,
}

impl AcpSession {
    pub fn new(cwd: String) -> Self {
        Self {
            cwd,
            session_id: uuid::Uuid::new_v4().to_string(),
            initialized: false,
        }
    }
}
