// crabjar/src/knowledge_store/mod.rs
// Bridge between state-docs and knowledge store

pub mod commands;

use std::path::Path;

use agent_context::state_docs::Annotation;
use agent_context::{KnowledgeEntry, KnowledgeKind, Source, Store};
use chrono::Utc;
use rusqlite::Connection;
use serde_json::json;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ConfidenceDefaults {
    pub note_confidence: f64,
    pub question_confidence: f64,
    pub promote_confidence: f64,
    pub provenance_id: String,
    pub set_at: u128,
    pub reason: String,
    pub source: String,
}

impl Default for ConfidenceDefaults {
    fn default() -> Self {
        Self {
            note_confidence: 0.80,
            question_confidence: 0.55,
            promote_confidence: 0.85,
            provenance_id: Uuid::new_v4().to_string(),
            set_at: now_unix_ms(),
            reason: "default confidence baselines".to_string(),
            source: "knowledge_store".to_string(),
        }
    }
}

#[allow(dead_code)]
impl ConfidenceDefaults {
    pub fn with_note_confidence(mut self, value: f64) -> Self {
        self.note_confidence = value;
        self.provenance_id = Uuid::new_v4().to_string();
        self.set_at = now_unix_ms();
        self
    }

    pub fn with_question_confidence(mut self, value: f64) -> Self {
        self.question_confidence = value;
        self.provenance_id = Uuid::new_v4().to_string();
        self.set_at = now_unix_ms();
        self
    }

    pub fn with_promote_confidence(mut self, value: f64) -> Self {
        self.promote_confidence = value;
        self.provenance_id = Uuid::new_v4().to_string();
        self.set_at = now_unix_ms();
        self
    }
}

/// Helper to produce a standard knowledge-response JSON object
pub fn knowledge_response(
    message: impl Into<String>,
    payload: serde_json::Value,
) -> serde_json::Value {
    let mut response = json!({
        "success": true,
        "message": message.into(),
        "payload": payload,
    });

    if let Some(payload_obj) = response["payload"].as_object().cloned()
        && let Some(response_obj) = response.as_object_mut()
    {
        for (key, value) in payload_obj {
            response_obj.insert(key, value);
        }
    }

    response
}

/// Result of a gated knowledge insert.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum GatedInsertResult {
    Inserted { id: i64 },
    Quarantined { id: i64 },
    DryRun,
}

/// Bridge between state-docs and knowledge store
#[allow(dead_code)]
pub struct KnowledgeBridge {
    knowledge_store: Store,
    project_root: PathBuf,
    mirror_log_conn: Option<Connection>,
    guard_db: Option<crabjar_guard::GuardDb>,
}

#[allow(dead_code)]
impl KnowledgeBridge {
    const STATE_DOC_SOURCE_TYPE: &'static str = "state_doc_annotation";

    pub fn new(
        knowledge_store_path: &str,
        project_root: impl Into<PathBuf>,
        mirror_log_db_path: Option<PathBuf>,
    ) -> Result<Self, agent_context::Error> {
        let conn = rusqlite::Connection::open(knowledge_store_path)?;
        agent_context::schema::migrate(&conn)?;
        let knowledge_store = Store { conn };
        let project_root = project_root.into();
        let mirror_log_conn = mirror_log_db_path.map(Connection::open).transpose()?;

        Ok(Self {
            knowledge_store,
            project_root,
            mirror_log_conn,
            guard_db: None,
        })
    }

    /// Attach a guard database for write authorization.
    pub fn with_guard_db(mut self, guard_db: crabjar_guard::GuardDb) -> Self {
        self.guard_db = Some(guard_db);
        self
    }

    /// Resolve a state-doc path from project root
    pub fn resolve_doc_path(&self, doc_name: &str) -> Result<PathBuf, agent_context::Error> {
        let docs_dir = self
            .project_root
            .join(agent_context::state_docs::STATE_DOCS_DIR);
        Ok(docs_dir.join(format!("{}.md", doc_name)))
    }

    /// Load overlay JSON for a state-doc path
    pub fn load_overlay_for_path(
        &self,
        path: &Path,
    ) -> Result<serde_json::Value, agent_context::Error> {
        let overlay_dir = path
            .parent()
            .unwrap()
            .join(agent_context::state_docs::OVERLAY_DIR);
        let stem = path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .trim_end_matches(".md")
            .to_string();
        let overlay_file = overlay_dir.join(format!("{}.overlay.json", stem));
        let content = std::fs::read_to_string(&overlay_file)
            .map_err(|e| agent_context::Error::Io(std::io::Error::other(e.to_string())))?;
        serde_json::from_str(&content).map_err(agent_context::Error::Json)
    }

    /// Convert an annotation to a knowledge entry
    pub fn annotation_to_knowledge(
        &self,
        annotation: &Annotation,
    ) -> Result<KnowledgeEntry, agent_context::Error> {
        let kind = match annotation.kind.as_str() {
            "note" => KnowledgeKind::Context,
            "question" => KnowledgeKind::Instruction,
            _ => KnowledgeKind::Context,
        };

        let defaults = ConfidenceDefaults::default();
        let confidence = annotation_confidence(annotation, &defaults);
        let provenance_id = Uuid::new_v4().to_string();
        let mut entry = KnowledgeEntry::new(&annotation.message, kind)
            .meta("source_id", annotation.id.to_string())
            .meta("source_doc", &annotation.doc_name)
            .meta("annotation_kind", annotation.kind.clone())
            .meta("confidence", confidence)
            .meta("derived_at_unix_ms", now_unix_ms())
            .meta("status", annotation.status.clone())
            .meta("provenance_id", &provenance_id)
            .meta("provenance_source", Self::STATE_DOC_SOURCE_TYPE)
            .meta("provenance_set_at_unix_ms", now_unix_ms());
        entry.source_type = Self::STATE_DOC_SOURCE_TYPE.to_string();
        entry.source_id = annotation.id.to_string();
        entry.provenance_id = provenance_id;
        entry.source = Source::Agent;
        entry.weight = confidence;
        let doc_name = annotation
            .doc_name
            .strip_suffix(".md")
            .unwrap_or(&annotation.doc_name);
        entry.tags = std::iter::once("state-doc".to_string())
            .chain(doc_name.split('_').map(|s| s.to_string()))
            .collect();
        Ok(entry)
    }

    /// Query knowledge entries by tags and optional source_doc filter
    pub fn query_state_docs(
        &self,
        tags: &[&str],
        limit: usize,
        source_doc: &str,
    ) -> Result<Vec<serde_json::Value>, agent_context::Error> {
        let rows = self
            .knowledge_store
            .query(tags, limit, "", source_doc, "")?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let mut meta = row.metadata;
                if let Some(source_id) = meta.get("source_id").cloned()
                    && let Some(meta_obj) = meta.as_object_mut()
                {
                    meta_obj
                        .entry("annotation_id".to_string())
                        .or_insert(source_id);
                }
                json!({
                    "id": row.id,
                    "content": row.content,
                    "tags": row.tags,
                    "meta": meta.clone(),
                    "metadata": meta,
                    "active": row.active,
                })
            })
            .collect())
    }

    /// Sync all open annotations for a state-doc into the knowledge store
    pub fn sync_state_doc_annotations(
        &self,
        doc_name: &str,
    ) -> Result<Vec<i64>, agent_context::Error> {
        let overlay = self.load_overlay_for_path(&self.resolve_doc_path(doc_name)?)?;

        let mut ids = Vec::new();
        if let Some(entries) = overlay.get("entries").and_then(|v| v.as_array()) {
            for entry in entries {
                let status = entry.get("status").and_then(|v| v.as_str()).unwrap_or("");
                if status != "open" {
                    continue;
                }
                let id_str = entry.get("id").and_then(|v| v.as_str()).unwrap_or("");
                if id_str.is_empty() {
                    continue;
                }
                if self
                    .knowledge_store
                    .find_active_by_source(Self::STATE_DOC_SOURCE_TYPE, id_str)?
                    .is_some()
                {
                    continue;
                }
                let _annotation_id = id_str.parse::<i64>().unwrap_or(0);
                let message = entry.get("message").and_then(|v| v.as_str()).unwrap_or("");
                let kind_str = entry.get("kind").and_then(|v| v.as_str()).unwrap_or("note");
                let kind = match kind_str {
                    "note" => KnowledgeKind::Context,
                    "question" => KnowledgeKind::Instruction,
                    _ => KnowledgeKind::Context,
                };
                let mut knowledge = KnowledgeEntry::new(message, kind)
                    .meta("source_id", id_str)
                    .meta("source_doc", doc_name)
                    .meta("annotation_kind", kind_str)
                    .meta("confidence", 0.80)
                    .meta("derived_at_unix_ms", now_unix_ms())
                    .meta("status", "active");
                knowledge.source_type = Self::STATE_DOC_SOURCE_TYPE.to_string();
                knowledge.source_id = id_str.to_string();
                knowledge.provenance_id = Uuid::new_v4().to_string();
                knowledge.source = Source::Agent;
                knowledge.weight = 0.80;
                knowledge.tags = std::iter::once("state-doc".to_string())
                    .chain(doc_name.split('_').map(|s| s.to_string()))
                    .collect();
                let id = self.knowledge_store.insert(knowledge)?;
                ids.push(id);
            }
        }

        Ok(ids)
    }

    /// List all state-docs that have synced annotations in the knowledge store
    pub fn list_synced_state_docs(&self) -> Result<Vec<String>, agent_context::Error> {
        let docs_dir = self
            .project_root
            .join(agent_context::state_docs::STATE_DOCS_DIR);
        let mut synced = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&docs_dir) {
            for entry in entries {
                if let Ok(e) = entry
                    && let Some(name) = e.file_name().to_string_lossy().strip_suffix(".md")
                {
                    let overlay_path = e
                        .path()
                        .parent()
                        .unwrap()
                        .join(agent_context::state_docs::OVERLAY_DIR)
                        .join(format!("{}.overlay.json", name));
                    if overlay_path.exists() {
                        synced.push(name.to_string());
                    }
                }
            }
        }
        Ok(synced)
    }

    /// Get knowledge entries associated with a specific state-doc
    pub fn get_state_doc_knowledge(
        &self,
        doc_name: &str,
    ) -> Result<Vec<serde_json::Value>, agent_context::Error> {
        let overlay = self.load_overlay_for_path(&self.resolve_doc_path(doc_name)?)?;

        let tags: Vec<&str> = overlay
            .get("entries")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|e| e.get("doc").and_then(|v| v.as_str()))
                    .collect()
            })
            .unwrap_or_default();

        self.query_state_docs(&tags, 100, doc_name)
    }

    /// Get recent events from the knowledge store's event log
    pub fn get_events(&self, limit: usize) -> Result<Vec<serde_json::Value>, agent_context::Error> {
        let rows = self.knowledge_store.events(limit)?;
        if rows.is_empty() {
            return Ok(vec![]);
        }
        Ok(rows
            .into_iter()
            .map(
                |row| json!({ "id": row.id, "event_type": row.event_type, "timestamp": row.timestamp }),
            )
            .collect())
    }

    /// Deactivates knowledge derived from a resolved annotation.
    pub fn deactivate_annotation_knowledge(
        &self,
        annotation_id: &str,
        reason: Option<&str>,
    ) -> Result<usize, agent_context::Error> {
        Ok(self.knowledge_store.deactivate_by_source(
            Self::STATE_DOC_SOURCE_TYPE,
            annotation_id,
            Source::Agent,
            reason,
        )?)
    }

    /// Deactivates all knowledge entries by provenance_id across all provenance sources.
    pub fn deactivate_by_provenance_id(
        &self,
        provenance_id: &str,
        reason: Option<&str>,
    ) -> Result<usize, agent_context::Error> {
        Ok(self.knowledge_store.deactivate_by_provenance_id(
            provenance_id,
            Source::Agent,
            reason,
        )?)
    }

    /// Insert a standalone knowledge entry.
    pub fn insert_entry(
        &self,
        content: &str,
        kind: KnowledgeKind,
        tags: Vec<String>,
    ) -> Result<i64, agent_context::Error> {
        let mut entry = KnowledgeEntry::new(content, kind);
        entry.source = Source::User;
        entry.source_type = "user".to_string();
        entry.tags = tags;
        Ok(self.knowledge_store.insert(entry)?)
    }

    /// Insert a knowledge entry gated by source type.
    /// External sources are routed to quarantine (pending) regardless of confidence.
    pub fn insert_gated(
        &self,
        content: &str,
        kind: KnowledgeKind,
        tags: Vec<String>,
        source: Source,
    ) -> Result<GatedInsertResult, agent_context::Error> {
        let source_str = match source {
            Source::User => "user",
            Source::Agent => "agent",
            Source::System => "system",
            Source::External => "external",
        };

        let gate_result = if let Some(guard_db) = &self.guard_db {
            let gate = crabjar_guard::ExecutionGate::new(guard_db, false, &self.project_root);
            gate.check_knowledge_write(source_str)
                .map_err(|e| agent_context::Error::Internal(e.to_string()))?
        } else {
            crabjar_guard::GateResult::Proceed
        };

        let entry = {
            let mut entry = KnowledgeEntry::new(content, kind);
            entry.source = source;
            entry.source_type = source_str.to_string();
            entry.tags = tags;
            entry
        };

        match gate_result {
            crabjar_guard::GateResult::Proceed => {
                let id = self.knowledge_store.insert(entry)?;
                Ok(GatedInsertResult::Inserted { id })
            }
            crabjar_guard::GateResult::Pending => {
                let mut pending_entry = entry;
                let mut new_meta = pending_entry
                    .metadata
                    .as_object()
                    .cloned()
                    .unwrap_or_else(serde_json::Map::new);
                new_meta.insert("quarantined".to_string(), serde_json::json!(true));
                new_meta.insert(
                    "quarantined_at".to_string(),
                    serde_json::json!(chrono::Utc::now().to_rfc3339()),
                );
                pending_entry.metadata = serde_json::Value::Object(new_meta);
                let id = self.knowledge_store.insert(pending_entry)?;
                Ok(GatedInsertResult::Quarantined { id })
            }
            crabjar_guard::GateResult::Interrupted { reason } => {
                Err(agent_context::Error::Internal(format!(
                    "Knowledge write blocked: {}",
                    reason
                )))
            }
            crabjar_guard::GateResult::DryRun => {
                Ok(GatedInsertResult::DryRun)
            }
            crabjar_guard::GateResult::Revoked { reason } => {
                Err(agent_context::Error::Internal(format!(
                    "Knowledge write revoked: {}",
                    reason
                )))
            }
        }
    }

    /// Verify knowledge store integrity.
    pub fn verify(&self) -> Result<Vec<i64>, agent_context::Error> {
        Ok(self.knowledge_store.verify()?)
    }

    /// Deactivate a knowledge entry by ID.
    pub fn deactivate(
        &self,
        id: i64,
        source: Source,
        reason: Option<&str>,
    ) -> Result<(), agent_context::Error> {
        Ok(self.knowledge_store.deactivate(id, source, reason)?)
    }

    /// Promote a quarantined knowledge entry to active.
    /// Removes the quarantine flag and sets the entry as active.
    pub fn promote_quarantined(&self, id: i64, reason: &str) -> Result<bool, agent_context::Error> {
        let conn = &self.knowledge_store.conn;
        
        let is_quarantined: bool = conn.query_row(
            "SELECT COALESCE(metadata->>'$.quarantined', 'false') = 'true' FROM knowledge_entries WHERE id = ?",
            [id],
            |row| row.get(0),
        ).unwrap_or(false);

        if !is_quarantined {
            return Err(agent_context::Error::Internal(format!(
                "Entry {} is not quarantined", id
            )));
        }

        conn.execute(
            "UPDATE knowledge_entries SET active = 1, metadata = json_set(
                metadata, '$.quarantined', 0,
                '$.promoted_at', ?,
                '$.promotion_reason', ?
            ) WHERE id = ?",
            rusqlite::params![
                chrono::Utc::now().to_rfc3339(),
                reason,
                id,
            ],
        )?;

        let ts = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO event_rows (event_type, timestamp) VALUES (?, ?)",
            rusqlite::params!["promote", ts],
        )?;

        Ok(true)
    }

    /// List quarantined entries (for human review).
    pub fn list_quarantined(&self) -> Result<Vec<serde_json::Value>, agent_context::Error> {
        let mut stmt = self.knowledge_store.conn.prepare(
            "SELECT id, content, tags, metadata, source, source_type, created_at 
             FROM knowledge_entries 
             WHERE metadata->>'$.quarantined' = 'true' 
             ORDER BY created_at DESC"
        )?;
        
        let mut rows = Vec::new();
        let cursor = stmt.query_map([], |row| {
            Ok((
                row.get::<usize, i64>(0)?,
                row.get::<usize, String>(1)?,
                row.get::<usize, String>(2)?,
                row.get::<usize, String>(3)?,
                row.get::<usize, String>(4)?,
                row.get::<usize, String>(5)?,
                row.get::<usize, String>(6)?,
            ))
        })?;
        
        for row in cursor {
            let (id, content, tags, metadata, source, source_type, created_at) = row?;
            rows.push(json!({
                "id": id,
                "content": content,
                "tags": serde_json::from_str::<Vec<String>>(&tags).unwrap_or_default(),
                "metadata": serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&metadata).unwrap_or_default(),
                "source": source,
                "source_type": source_type,
                "created_at": created_at,
            }));
        }
        
        Ok(rows)
    }

    /// Resolve an annotation and deactivate derived knowledge entries.
    /// Updates the overlay file to mark the annotation as resolved.
    pub fn resolve_annotation(
        &self,
        doc_name: &str,
        annotation_id: &str,
        reason: &str,
    ) -> Result<(usize, Annotation), agent_context::Error> {
        let overlay_path = self.resolve_doc_path(doc_name)?;
        let overlay_dir = overlay_path
            .parent()
            .unwrap()
            .join(agent_context::state_docs::OVERLAY_DIR);
        let overlay_file = overlay_dir.join(format!(
            "{}.overlay.json",
            overlay_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .trim_end_matches(".md")
        ));

        let mut overlay = self.load_overlay_for_path(&overlay_path)?;

        let resolved_entry = overlay
            .get("entries")
            .and_then(|v| v.as_array())
            .and_then(|a| {
                a.iter()
                    .find(|e| e.get("id").and_then(|v| v.as_str()).unwrap_or("") == annotation_id)
            })
            .ok_or_else(|| {
                agent_context::Error::Internal(format!("annotation not found: {}", annotation_id))
            })?;

        let id = resolved_entry
            .get("id")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let source_doc = resolved_entry
            .get("doc")
            .and_then(|v| v.as_str())
            .unwrap_or(doc_name)
            .to_string();
        let section_id = resolved_entry.get("section_id").and_then(|v| v.as_i64());
        let line = resolved_entry
            .get("line")
            .and_then(|v| v.as_u64())
            .map(|l| l as usize)
            .unwrap_or(0);
        let kind = resolved_entry
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("note")
            .to_string();
        let message = resolved_entry
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let author = resolved_entry
            .get("author")
            .and_then(|v| v.as_str())
            .unwrap_or("agent")
            .to_string();
        let created_at = resolved_entry
            .get("created_at_unix_ms")
            .and_then(|v| v.as_u64())
            .map(|ms| {
                chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms as i64)
                    .unwrap_or(Utc::now())
            })
            .unwrap_or(Utc::now());

        // Update overlay: mark as resolved, add reason
        if let Some(entries) = overlay.get_mut("entries").and_then(|v| v.as_array_mut()) {
            for entry in entries.iter_mut() {
                if entry.get("id").and_then(|v| v.as_str()).unwrap_or("") == annotation_id {
                    if let Some(obj) = entry.as_object_mut() {
                        obj.insert("status".to_string(), serde_json::json!("resolved"));
                        obj.insert(
                            "resolved_at_unix_ms".to_string(),
                            serde_json::json!(now_unix_ms()),
                        );
                        obj.insert("resolution_reason".to_string(), serde_json::json!(reason));
                    }
                    break;
                }
            }
        }

        // Write updated overlay back
        std::fs::create_dir_all(&overlay_dir).ok();
        let json = serde_json::to_string_pretty(&overlay).map_err(agent_context::Error::Json)?;
        std::fs::write(&overlay_file, json)
            .map_err(|e| agent_context::Error::Io(std::io::Error::other(e.to_string())))?;

        let deactivated = self.deactivate_annotation_knowledge(annotation_id, Some(reason))?;

        let resolved = Annotation {
            id,
            doc_name: source_doc,
            section_id,
            line,
            kind,
            message,
            author,
            status: "resolved".to_string(),
            created_at,
        };
        Ok((deactivated, resolved))
    }

    /// Promote a raw event from mirror-log to a knowledge entry
    pub fn promote_event(&self, event_id: i64) -> Result<String, agent_context::Error> {
        let conn = self.mirror_log_conn.as_ref().ok_or_else(|| {
            agent_context::Error::Internal("mirror-log connection not available".to_string())
        })?;

        let id_str = event_id.to_string();

        let (content, _source, meta): (String, String, Option<String>) = conn
            .query_row(
                "SELECT content, source, meta FROM events WHERE id = ?1",
                [&id_str],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|e| agent_context::Error::Internal(format!("Failed to find event: {}", e)))?;

        let provenance_id = Uuid::new_v4().to_string();
        let defaults = ConfidenceDefaults::default();
        let mut entry = KnowledgeEntry::new(&content, KnowledgeKind::Context);
        entry.source_type = "mirror_log_event".to_string();
        entry.source_id = event_id.to_string();
        entry.source = Source::Agent;
        entry = entry
            .meta("confidence", defaults.promote_confidence)
            .meta("derived_at_unix_ms", now_unix_ms())
            .meta("status", "active")
            .meta("provenance_id", provenance_id)
            .meta("provenance_source", "mirror_log_event")
            .meta("provenance_set_at_unix_ms", now_unix_ms());
        if let Some(m) = meta {
            entry = entry.meta("event-meta", json!(m));
        }

        let new_id = self.knowledge_store.insert(entry)?;
        Ok(new_id.to_string())
    }
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn annotation_confidence(annotation: &Annotation, defaults: &ConfidenceDefaults) -> f64 {
    let base = match annotation.kind.as_str() {
        "note" => defaults.note_confidence,
        "question" => defaults.question_confidence,
        _ => defaults.note_confidence,
    };

    let message = annotation.message.to_ascii_lowercase();
    let mut confidence: f64 = base;

    for marker in [
        "maybe",
        "might",
        "should",
        "todo",
        "follow-up",
        "follow up",
        "?",
    ] {
        if message.contains(marker) {
            confidence -= 0.10;
        }
    }

    confidence.clamp(0.20, 0.95)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn sync_is_idempotent_on_open_annotations() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("knowledge.db");
        let db_path_str = db_path.to_string_lossy().into_owned();
        let bridge = KnowledgeBridge::new(&db_path_str, dir.path(), None).unwrap();

        let state_docs = dir.path().join("state-docs");
        fs::create_dir_all(&state_docs).unwrap();
        let overlay = state_docs.join("overlay");
        fs::create_dir_all(&overlay).unwrap();
        fs::write(state_docs.join("alpha.md"), "# Alpha\n").unwrap();
        fs::write(
            overlay.join("alpha.overlay.json"),
            r#"{
  "entries": [
    {
      "id": "123",
      "kind": "note",
      "message": "Keep this",
      "author": "agent",
      "doc": "alpha.md",
      "line": null,
      "status": "open",
      "created_at_unix_ms": 123
    }
  ]
}"#,
        )
        .unwrap();

        let first_ids = bridge.sync_state_doc_annotations("alpha").unwrap();
        assert_eq!(first_ids.len(), 1);

        let second_ids = bridge.sync_state_doc_annotations("alpha").unwrap();
        assert_eq!(second_ids.len(), 0);
    }

    #[test]
    fn deactivate_resolved_annotation_knowledge_returns_count() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("knowledge.db");
        let db_path_str = db_path.to_string_lossy().into_owned();
        let bridge = KnowledgeBridge::new(&db_path_str, dir.path(), None).unwrap();

        let state_docs = dir.path().join("state-docs");
        fs::create_dir_all(&state_docs).unwrap();
        let overlay = state_docs.join("overlay");
        fs::create_dir_all(&overlay).unwrap();
        fs::write(state_docs.join("beta.md"), "# Beta\n").unwrap();
        fs::write(
            overlay.join("beta.overlay.json"),
            r#"{
  "entries": [
    {
      "id": "456",
      "kind": "question",
      "message": "Decided yes",
      "author": "agent",
      "doc": "beta.md",
      "line": null,
      "status": "open",
      "created_at_unix_ms": 456
    }
  ]
}"#,
        )
        .unwrap();

        let ids = bridge.sync_state_doc_annotations("beta").unwrap();
        assert_eq!(ids.len(), 1);

        let deactivated = bridge
            .deactivate_annotation_knowledge("456", Some("answered"))
            .unwrap();
        assert_eq!(deactivated, 1);

        let rows = bridge
            .query_state_docs(&["state-doc", "beta"], 100, "beta.md")
            .unwrap();
        assert_eq!(rows.len(), 0);
    }
}
