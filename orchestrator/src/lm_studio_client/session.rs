//! Session state management and SQLite-backed session persistence.

use super::types::{MessageRole, UnifiedChatResponse, UnifiedMessage};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Session state management
// ---------------------------------------------------------------------------

/// Tracks session state for stateful chat continuation.
///
/// For the native endpoint, this stores the `response_id` so the next
/// request can use `previous_response_id` to continue the conversation.
///
/// For OpenAI/Anthropic endpoints, this stores the full message history
/// so it can be re-sent on each turn.
#[derive(Debug, Clone)]
pub struct SessionState {
    /// The current response ID (native endpoint).
    pub response_id: Option<String>,
    /// The full message history (OpenAI/Anthropic endpoints).
    pub message_history: Vec<UnifiedMessage>,
}

impl SessionState {
    /// Creates a new empty session.
    pub fn new() -> Self {
        Self {
            response_id: None,
            message_history: Vec::new(),
        }
    }

    /// Initializes a session with a system message.
    pub fn with_system_prompt(system_prompt: String) -> Self {
        Self {
            response_id: None,
            message_history: vec![UnifiedMessage {
                role: MessageRole::System,
                content: system_prompt,
            }],
        }
    }

    /// Updates the session state with a response from the model.
    pub fn update_with_response(&mut self, response: &UnifiedChatResponse) {
        // Store the response ID for stateful continuation.
        if let Some(ref rid) = response.response_id {
            self.response_id = Some(rid.clone());
        }

        // Collect assistant messages from the response output.
        for item in &response.output {
            match item {
                crate::types::UnifiedOutputItem::Message { content } => {
                    self.message_history.push(UnifiedMessage {
                        role: MessageRole::Assistant,
                        content: content.clone(),
                    });
                }
                crate::types::UnifiedOutputItem::ToolCall {
                    tool,
                    output: Some(result),
                    ..
                } => {
                    self.message_history.push(UnifiedMessage {
                        role: MessageRole::Assistant,
                        content: format!("Tool '{}' executed: {}", tool, result),
                    });
                }
                crate::types::UnifiedOutputItem::Reasoning { content } => {
                    self.message_history.push(UnifiedMessage {
                        role: MessageRole::Assistant,
                        content: format!("[reasoning] {}", content),
                    });
                }
                _ => {}
            }
        }
    }

    /// Adds a user message to the session.
    pub fn add_user_message(&mut self, content: String) {
        self.message_history.push(UnifiedMessage {
            role: MessageRole::User,
            content,
        });
    }

    /// Returns whether this session has a response ID for stateful continuation.
    pub fn has_response_id(&self) -> bool {
        self.response_id.is_some()
    }
}

// ---------------------------------------------------------------------------
// SQLite-backed session store
// ---------------------------------------------------------------------------

/// SQLite-backed session store for persisting chat state across process restarts.
///
/// This complements LM Studio's native stateful chat by storing session
/// metadata and message history in SQLite. When the orchestrator restarts,
/// it can restore sessions from the database.
#[derive(Debug)]
pub struct SessionStore {
    /// SQLite database path.
    pub db_path: String,
    /// Connection to the database (opened lazily).
    conn: std::sync::Mutex<Option<rusqlite::Connection>>,
}

impl SessionStore {
    /// Creates a new session store with the given database path.
    pub fn new(db_path: String) -> Self {
        Self {
            db_path,
            conn: std::sync::Mutex::new(None),
        }
    }

    /// Opens the database connection lazily.
    fn open_conn(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, Option<rusqlite::Connection>>, SessionError> {
        let mut guard = self
            .conn
            .lock()
            .map_err(|e| SessionError::Database(format!("failed to lock connection: {e}")))?;
        if guard.is_none() {
            let conn = rusqlite::Connection::open(&self.db_path)
                .map_err(|e| SessionError::Database(format!("failed to open DB: {e}")))?;
            conn.execute_batch(
                "PRAGMA journal_mode=WAL;
                 CREATE TABLE IF NOT EXISTS sessions (
                     id TEXT PRIMARY KEY,
                     system_prompt TEXT,
                     message_history TEXT,
                     response_id TEXT,
                     created_at INTEGER DEFAULT (unixepoch()),
                     updated_at INTEGER DEFAULT (unixepoch())
                 );
                 CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(updated_at DESC);",
            )
            .map_err(|e| SessionError::Database(format!("failed to init schema: {e}")))?;
            *guard = Some(conn);
        }
        Ok(guard)
    }

    /// Creates a new session and returns its ID.
    ///
    /// The session is initialized with a system prompt.
    pub fn create_session(&self, system_prompt: Option<String>) -> Result<String, SessionError> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let system_prompt_json =
            serde_json::to_string(&system_prompt).unwrap_or_else(|_| "null".to_string());

        let guard = self.open_conn()?;
        let conn = guard.as_ref().unwrap();
        conn.execute(
            "INSERT INTO sessions (id, system_prompt, message_history, response_id) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                &session_id,
                system_prompt_json,
                "[]", // empty message history
                "",   // no response_id yet
            ],
        )
        .map_err(|e| SessionError::Database(format!("failed to create session: {e}")))?;

        debug!(
            "Created session {} with system prompt: {:?}",
            session_id, system_prompt
        );

        Ok(session_id)
    }

    /// Retrieves session state by ID.
    pub fn get_session(&self, session_id: &str) -> Result<SessionState, SessionError> {
        let guard = self.open_conn()?;
        let conn = guard.as_ref().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT system_prompt, message_history, response_id FROM sessions WHERE id = ?1",
            )
            .map_err(|e| SessionError::Database(format!("failed to prepare query: {e}")))?;

        let row = stmt
            .query_row(rusqlite::params![session_id], |row| {
                let system_prompt: String = row.get(0)?;
                let message_history: String = row.get(1)?;
                let response_id: String = row.get(2)?;
                Ok((system_prompt, message_history, response_id))
            })
            .ok(); // stub: return None for missing sessions

        let (system_prompt, message_history, response_id) = match row {
            Some(r) => (r.0, r.1, r.2),
            None => (String::new(), String::new(), String::new()),
        };

        let _system_prompt: Option<String> = serde_json::from_str(&system_prompt).unwrap_or(None);
        let message_history: Vec<UnifiedMessage> =
            serde_json::from_str(&message_history).unwrap_or_default();
        let response_id = if response_id.is_empty() {
            None
        } else {
            Some(response_id)
        };

        let mut state = SessionState::new();
        state.response_id = response_id;
        for msg in message_history {
            state.message_history.push(msg);
        }

        debug!("Retrieved session {}", session_id);
        Ok(state)
    }

    /// Updates session state in the store.
    pub fn update_session(
        &self,
        session_id: &str,
        state: &SessionState,
    ) -> Result<(), SessionError> {
        let message_history_json =
            serde_json::to_string(&state.message_history).unwrap_or_else(|_| "[]".to_string());
        let response_id = state.response_id.as_deref().unwrap_or("");

        let guard = self.open_conn()?;
        let conn = guard.as_ref().unwrap();
        conn.execute(
            "UPDATE sessions SET message_history = ?1, response_id = ?2, updated_at = unixepoch() WHERE id = ?3",
            rusqlite::params![message_history_json, response_id, session_id],
        )
        .map_err(|e| SessionError::Database(format!("failed to update session: {e}")))?;

        debug!("Updated session {}", session_id);
        Ok(())
    }

    /// Deletes a session. Idempotent — returns Ok even if the session doesn't exist.
    pub fn delete_session(&self, session_id: &str) -> Result<(), SessionError> {
        let guard = self.open_conn()?;
        let conn = guard.as_ref().unwrap();
        let _rows = conn
            .execute(
                "DELETE FROM sessions WHERE id = ?1",
                rusqlite::params![session_id],
            )
            .map_err(|e| SessionError::Database(format!("failed to delete session: {e}")))?;

        debug!("Deleted session {}", session_id);
        Ok(())
    }
}

/// Errors from session store operations.
#[derive(Debug, Error)]
pub enum SessionError {
    #[error("session not found: {0}")]
    NotFound(String),
    #[error("database error: {0}")]
    Database(String),
}
