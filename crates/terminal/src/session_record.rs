//! Native JSONL session record (ADR-005) — the faithful on-disk form.
//!
//! Line 1 is the header (`{"version": N, "session": ..., "backend": ...}`);
//! every following line is one `TerminalEvent`. asciinema v2 is a lossy
//! projection of this; this is the source of truth on disk.
//!
//! Versioning: `version` in the header is the stream format version
//! ([`STREAM_VERSION`]). v1 events lack the per-event `at` timestamp;
//! parsing defaults it to the epoch (v1 records carried no usable per-event
//! time — the asciinema serializer cannot recover it).

use std::fs;

use serde::{Deserialize, Serialize};

use crate::stream::TerminalEvent;

/// A versioned, on-disk session record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    /// Stream format version.
    pub version: u32,
    /// Session name.
    pub session: String,
    /// Backend name (herdr, wezterm, zellij).
    pub backend: String,
    /// The events, in submission order.
    pub events: Vec<TerminalEvent>,
}

impl SessionRecord {
    /// Serialize to native JSONL (header line + one event per line).
    pub fn to_jsonl(&self) -> String {
        let mut out = String::new();
        let header = serde_json::json!({
            "version": self.version,
            "session": self.session,
            "backend": self.backend,
        });
        out.push_str(&header.to_string());
        out.push('\n');
        for event in &self.events {
            out.push_str(&event.to_json_line());
            out.push('\n');
        }
        out
    }

    /// Parse native JSONL (header line + one event per line).
    ///
    /// Accepts v1 and v2 records: v1 events lack `at`, which serde
    /// defaults to the epoch.
    pub fn from_jsonl(text: &str) -> anyhow::Result<Self> {
        let mut lines = text.lines();
        let header_line = lines
            .next()
            .ok_or_else(|| anyhow::anyhow!("empty session record"))?;
        let header: serde_json::Value = serde_json::from_str(header_line)?;
        let version = header
            .get("version")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("session record header missing version"))?
            as u32;
        let session = header
            .get("session")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("session record header missing session"))?
            .to_string();
        let backend = header
            .get("backend")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let events = lines
            .filter(|l| !l.trim().is_empty())
            .map(|line| {
                serde_json::from_str::<TerminalEvent>(line)
                    .map_err(|e| anyhow::anyhow!("bad event line: {e}"))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(Self {
            version,
            session,
            backend,
            events,
        })
    }

    /// Write the record to `path` as native JSONL.
    pub fn save(&self, path: &std::path::Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.to_jsonl())?;
        Ok(())
    }

    /// Read a record from `path` (native JSONL).
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        Self::from_jsonl(&fs::read_to_string(path)?)
    }
}

impl TerminalEvent {
    /// Serialize a single event to one JSONL line.
    pub fn to_json_line(&self) -> String {
        serde_json::to_string(self).expect("TerminalEvent serialization cannot fail")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::{Receipt, STREAM_VERSION, SessionStream};
    use chrono::{DateTime, Utc};
    use std::time::Duration;

    fn receipt(cmd: &str, out: &str, code: i32) -> Receipt {
        Receipt {
            command: cmd.to_string(),
            output: out.to_string(),
            exit_code: Some(code),
            duration: Duration::from_millis(42),
            cwd: Some("/tmp".to_string()),
        }
    }

    #[test]
    fn jsonl_round_trip_preserves_events() {
        let mut s = SessionStream::new();
        s.push_receipt(&receipt("echo a", "a", 0));
        s.push(TerminalEvent::Raw {
            id: 0,
            data: "x".into(),
            at: Utc::now(),
        });
        let record = SessionRecord {
            version: STREAM_VERSION,
            session: "test".into(),
            backend: "herdr".into(),
            events: s.events().to_vec(),
        };
        let jsonl = record.to_jsonl();
        let parsed = SessionRecord::from_jsonl(&jsonl).expect("parse");
        assert_eq!(parsed.version, STREAM_VERSION);
        assert_eq!(parsed.session, "test");
        assert_eq!(parsed.backend, "herdr");
        assert_eq!(parsed.events, record.events);
    }

    #[test]
    fn v1_record_parses_with_epoch_at() {
        // v1 events have no `at` field — the migration default.
        let v1 = [
            r#"{"version":1,"session":"old","backend":"herdr"}"#,
            r#"{"type":"prompt","id":0,"cwd":"/tmp"}"#,
            r#"{"type":"command","id":1,"text":"echo a","started_at":"2026-08-30T00:00:00Z"}"#,
            r#"{"type":"output","id":2,"data":"a","exit_code":0}"#,
        ]
        .join("\n");
        let parsed = SessionRecord::from_jsonl(&v1).expect("parse v1");
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.events.len(), 3);
        // v1 `at` defaults to the epoch.
        if let TerminalEvent::Prompt { at, cwd, .. } = &parsed.events[0] {
            assert_eq!(*at, DateTime::from_timestamp(0, 0).unwrap());
            assert_eq!(cwd.as_deref(), Some("/tmp"));
        } else {
            panic!("v1 prompt event mis-parsed: {:?}", parsed.events[0]);
        }
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("sub/session.jsonl");
        let record = SessionRecord {
            version: STREAM_VERSION,
            session: "s".into(),
            backend: "herdr".into(),
            events: vec![],
        };
        record.save(&path).expect("save");
        let loaded = SessionRecord::load(&path).expect("load");
        assert_eq!(loaded.session, "s");
        assert!(loaded.events.is_empty());
    }
}
