//! Typed terminal event stream (ADR-005).
//!
//! The `TerminalEvent` stream is the substrate: append-only, monotonic ids,
//! and the source of truth for a recorded session. asciinema v2, WebSocket
//! JSON, and native JSONL are serializers of the stream, never the model.
//!
//! ## Event vocabulary
//!
//! ```text
//! Prompt  { id, cwd? }
//! Command { id, text, started_at }
//! Output  { id, data, exit_code? }
//! Raw     { id, data }        // fallback when segmentation fails
//! ```
//!
//! `Raw` is the escape hatch: when segmentation can't cleanly split a
//! command from its output, the bytes land in `Raw` rather than corrupting a
//! `Command`/`Output` boundary. A session is never "unrecordable" — worst
//! case it is all `Raw`.
//!
//! ## Blocks
//!
//! Events group into **blocks** (prompt → command → output units). A block
//! is the addressable cell and the copy-paste unit. A completed block
//! yields a [`Receipt`] — `{ command, output, exit_code, duration, cwd }` —
//! the input shape the deterministic task scorers consume.

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Stream format version. Bump with a migration path when the event
/// vocabulary changes (same discipline as the state-docs schema).
pub const STREAM_VERSION: u32 = 1;

/// A single terminal event. Each event carries a monotonic id (same shape
/// as the guard's append-only event store) and a timestamp.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TerminalEvent {
    /// A shell prompt was observed. `cwd` is set when the backend reports it.
    Prompt {
        id: u64,
        at: DateTime<Utc>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
    },
    /// A command was submitted to the shell.
    Command {
        id: u64,
        text: String,
        started_at: DateTime<Utc>,
    },
    /// Command output. `exit_code` is set when the backend reports it.
    Output {
        id: u64,
        data: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        exit_code: Option<i32>,
    },
    /// Unsegmented bytes — the escape hatch when segmentation fails.
    Raw { id: u64, data: String },
}

/// A prompt → command → output unit. The addressable cell and the
/// copy-paste unit of a recorded session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// Monotonic id of the first event in the block.
    pub first_event_id: u64,
    /// Monotonic id of the last event in the block.
    pub last_event_id: u64,
    /// The submitted command text (absent for prompt-only or raw blocks).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// The captured output (trimmed of prompt lines and sentinels).
    pub output: String,
    /// Exit code, when the backend reported one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Working directory at submission time, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
}

/// The receipt: the input shape the deterministic task scorers consume
/// (`exit_code` / `file_exists` / `regex_match` / `json_path` verifiers).
///
/// This is the payoff of the typed stream — agents stop scraping output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Receipt {
    /// The submitted command text.
    pub command: String,
    /// The captured output (stdout + stderr, trimmed of prompt lines).
    pub output: String,
    /// Process exit code, when the backend reported one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Wall-clock duration of the command.
    pub duration: Duration,
    /// Working directory at submission time, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
}

/// An append-only terminal event stream with monotonic id assignment.
#[derive(Debug, Default)]
pub struct SessionStream {
    events: Vec<TerminalEvent>,
    next_id: u64,
}

impl SessionStream {
    /// Create an empty stream.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an event, assigning the next monotonic id.
    ///
    /// The id is assigned here — callers never mint ids themselves, which
    /// keeps the stream's monotonicity invariant mechanical.
    pub fn push(&mut self, event: TerminalEvent) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let stamped = match event {
            TerminalEvent::Prompt { at, cwd, .. } => TerminalEvent::Prompt {
                id,
                at,
                cwd,
            },
            TerminalEvent::Command { text, started_at, .. } => TerminalEvent::Command {
                id,
                text,
                started_at,
            },
            TerminalEvent::Output { data, exit_code, .. } => TerminalEvent::Output {
                id,
                data,
                exit_code,
            },
            TerminalEvent::Raw { data, .. } => TerminalEvent::Raw { id, data },
        };
        self.events.push(stamped);
        id
    }

    /// The events, in submission order.
    pub fn events(&self) -> &[TerminalEvent] {
        &self.events
    }

    /// Append the events for a completed command: Prompt → Command → Output.
    ///
    /// Returns the ids of the events appended.
    pub fn push_receipt(&mut self, receipt: &Receipt) -> (u64, u64, u64) {
        let prompt_id = self.push(TerminalEvent::Prompt {
            id: 0,
            at: Utc::now(),
            cwd: receipt.cwd.clone(),
        });
        let command_id = self.push(TerminalEvent::Command {
            id: 0,
            text: receipt.command.clone(),
            started_at: Utc::now(),
        });
        let output_id = self.push(TerminalEvent::Output {
            id: 0,
            data: receipt.output.clone(),
            exit_code: receipt.exit_code,
        });
        (prompt_id, command_id, output_id)
    }

    /// Group consecutive Prompt → Command → Output triples into blocks.
    ///
    /// Non-triple events (stray Raw, orphan Prompt) are grouped as
    /// command-less blocks — the stream never loses data on grouping.
    pub fn blocks(&self) -> Vec<Block> {
        let mut blocks = Vec::new();
        let mut i = 0;
        while i < self.events.len() {
            if let (
                TerminalEvent::Prompt { id: pid, cwd, .. },
                Some(TerminalEvent::Command { id: _cid, text, .. }),
                Some(TerminalEvent::Output { id: oid, data, exit_code, .. }),
            ) = (
                &self.events[i],
                self.events.get(i + 1),
                self.events.get(i + 2),
            ) {
                blocks.push(Block {
                    first_event_id: *pid,
                    last_event_id: *oid,
                    command: Some(text.clone()),
                    output: data.clone(),
                    exit_code: *exit_code,
                    cwd: cwd.clone(),
                });
                i += 3;
            } else {
                // Orphan run: everything until the next triple start.
                let first = &self.events[i];
                let first_id = first.id();
                let mut j = i + 1;
                while j < self.events.len() && !starts_triple(&self.events, j) {
                    j += 1;
                }
                let last_id = self.events[j.saturating_sub(1)].id();
                let output = self.events[i..j]
                    .iter()
                    .map(|e| match e {
                        TerminalEvent::Output { data, .. } | TerminalEvent::Raw { data, .. } => {
                            data.clone()
                        }
                        TerminalEvent::Command { text, .. } => text.clone(),
                        TerminalEvent::Prompt { .. } => String::new(),
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                blocks.push(Block {
                    first_event_id: first_id,
                    last_event_id: last_id,
                    command: None,
                    output,
                    exit_code: None,
                    cwd: None,
                });
                i = j;
            }
        }
        blocks
    }
}

impl TerminalEvent {
    /// The event's monotonic id.
    pub fn id(&self) -> u64 {
        match self {
            TerminalEvent::Prompt { id, .. }
            | TerminalEvent::Command { id, .. }
            | TerminalEvent::Output { id, .. }
            | TerminalEvent::Raw { id, .. } => *id,
        }
    }
}

fn starts_triple(events: &[TerminalEvent], at: usize) -> bool {
    matches!(
        (events.get(at), events.get(at + 1), events.get(at + 2)),
        (
            Some(TerminalEvent::Prompt { .. }),
            Some(TerminalEvent::Command { .. }),
            Some(TerminalEvent::Output { .. })
        )
    )
}

/// A versioned, on-disk session record: the native JSONL form.
///
/// Line 1 is the header (`{"version": N, "session": ..., "backend": ...}`);
/// every following line is one `TerminalEvent`. This is the faithful
/// on-disk form — asciinema v2 is a lossy projection of it.
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
    fn stream_assigns_monotonic_ids() {
        let mut s = SessionStream::new();
        let (p1, c1, o1) = s.push_receipt(&receipt("echo a", "a", 0));
        let (p2, c2, o2) = s.push_receipt(&receipt("echo b", "b", 0));
        assert_eq!(p1, 0);
        assert_eq!(c1, 1);
        assert_eq!(o1, 2);
        assert_eq!(p2, 3);
        assert_eq!(c2, 4);
        assert_eq!(o2, 5);
    }

    #[test]
    fn blocks_group_prompt_command_output_triples() {
        let mut s = SessionStream::new();
        s.push_receipt(&receipt("echo a", "a", 0));
        s.push_receipt(&receipt("echo b", "b", 1));
        let blocks = s.blocks();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].command.as_deref(), Some("echo a"));
        assert_eq!(blocks[0].output, "a");
        assert_eq!(blocks[0].exit_code, Some(0));
        assert_eq!(blocks[0].first_event_id, 0);
        assert_eq!(blocks[0].last_event_id, 2);
        assert_eq!(blocks[1].command.as_deref(), Some("echo b"));
        assert_eq!(blocks[1].exit_code, Some(1));
        assert_eq!(blocks[1].first_event_id, 3);
    }

    #[test]
    fn raw_events_never_lost_on_grouping() {
        let mut s = SessionStream::new();
        s.push(TerminalEvent::Raw { id: 0, data: "???".into() });
        s.push_receipt(&receipt("echo a", "a", 0));
        let blocks = s.blocks();
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].command.is_none());
        assert_eq!(blocks[0].output, "???");
        assert_eq!(blocks[1].command.as_deref(), Some("echo a"));
    }

    #[test]
    fn jsonl_round_trip_preserves_events() {
        let mut s = SessionStream::new();
        s.push_receipt(&receipt("echo a", "a", 0));
        s.push(TerminalEvent::Raw { id: 0, data: "x".into() });
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
    fn receipt_serializes_with_optional_fields() {
        let r = receipt("echo a", "a", 0);
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"exit_code\":0"));
        assert!(json.contains("\"cwd\":\"/tmp\""));
        let none = Receipt {
            exit_code: None,
            cwd: None,
            ..r
        };
        let json = serde_json::to_string(&none).unwrap();
        assert!(!json.contains("exit_code"));
        assert!(!json.contains("cwd"));
    }
}
