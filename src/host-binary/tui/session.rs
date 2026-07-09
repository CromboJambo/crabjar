//! Session persistence for the TUI conversation history.
//!
//! Sessions are stored as JSON files in a local directory, keyed by UUID.

use super::app::Message;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

/// A single conversation session.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Session {
    pub id: String,
    pub messages: Vec<Message>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Store for managing TUI sessions on disk.
pub struct SessionStore {
    data_dir: PathBuf,
}

impl SessionStore {
    /// Create a new session store at the given directory.
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        let dir = data_dir.into();
        if !dir.exists() {
            fs::create_dir_all(&dir).ok();
        }
        Self { data_dir: dir }
    }

    /// Create a new session and return its ID.
    pub fn create(&self) -> Result<String, Box<dyn std::error::Error>> {
        let id = Uuid::new_v4().to_string();
        let session = Session {
            id: id.clone(),
            messages: Vec::new(),
            created_at: chrono::Utc::now(),
        };
        self.save(&id, &session.messages)?;
        Ok(id)
    }

    /// Load a session by ID.
    pub fn load(&self, id: &str) -> Result<Session, Box<dyn std::error::Error>> {
        let path = self.data_dir.join(format!("{}.json", id));
        if !path.exists() {
            return Err(format!("session not found: {}", id).into());
        }

        let content = fs::read_to_string(&path)?;
        // Parse messages from JSON array
        let messages: Vec<Message> = serde_json::from_str(&content)?;
        Ok(Session {
            id: id.to_string(),
            messages,
            created_at: chrono::Utc::now(),
        })
    }

    /// Save session messages to disk.
    pub fn save(
        &self,
        id: &str,
        messages: &[Message],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = self.data_dir.join(format!("{}.json", id));
        let content = serde_json::to_string_pretty(messages)?;
        fs::write(&path, content)?;
        Ok(())
    }

    /// List all session IDs.
    pub fn list_ids(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut ids = Vec::new();
        if !self.data_dir.exists() {
            return Ok(ids);
        }

        for entry in fs::read_dir(&self.data_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Some(stem) = path.file_stem() {
                    ids.push(stem.to_string_lossy().to_string());
                }
            }
        }

        Ok(ids)
    }
}
