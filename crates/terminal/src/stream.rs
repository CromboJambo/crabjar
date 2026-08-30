//! Typed terminal event stream (ADR-005).
//!
//! The `TerminalEvent` stream is the substrate: append-only, monotonic ids,
//! and the source of truth for a recorded session. asciinema v2, WebSocket
//! JSON, and native JSONL are serializers of the stream, never the model.
//!
//! ## Event vocabulary
//!
//! ```text
//! Prompt  { id, at, cwd? }
//! Command { id, text, started_at, at }
//! Output  { id, data, exit_code?, at }
//! Raw     { id, data, at }        // fallback when segmentation fails
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
//!
//! ## Copy-paste
//!
//! Copy = select an event id range or a block (see [`crate::copy_paste`]).
//! Paste = serialize the selection to a wire target (native JSONL or
//! asciinema v2). The native JSONL is the faithful form; asciinema v2 is a
//! lossy projection (its `i`/`o` events drop block ids and exit codes).

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Stream format version. Bump with a migration path when the event
/// vocabulary changes (same discipline as the state-docs schema).
///
/// v2 (2026-08-30): every event carries an `at` timestamp (needed for the
/// asciinema v2 serializer's relative times). v1 events lack the field;
/// `from_jsonl` defaults it to the epoch.
pub const STREAM_VERSION: u32 = 2;

/// A single terminal event. Each event carries a monotonic id (same shape
/// as the guard's append-only event store) and an `at` timestamp.
///
/// `at` is `#[serde(default)]` to the epoch so v1 records (no per-event
/// time) still parse — see [`crate::session_record`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TerminalEvent {
    /// A shell prompt was observed. `cwd` is set when the backend reports it.
    Prompt {
        id: u64,
        #[serde(default = "epoch")]
        at: DateTime<Utc>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
    },
    /// A command was submitted to the shell. `started_at` is the submission
    /// time; `at` is when the event was appended to the stream.
    Command {
        id: u64,
        text: String,
        started_at: DateTime<Utc>,
        #[serde(default = "epoch")]
        at: DateTime<Utc>,
    },
    /// Command output. `exit_code` is set when the backend reports it.
    Output {
        id: u64,
        data: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        exit_code: Option<i32>,
        #[serde(default = "epoch")]
        at: DateTime<Utc>,
    },
    /// Unsegmented bytes — the escape hatch when segmentation fails.
    Raw {
        id: u64,
        data: String,
        #[serde(default = "epoch")]
        at: DateTime<Utc>,
    },
}

/// Serde default for `at`: the epoch (v1 migration path).
fn epoch() -> DateTime<Utc> {
    DateTime::from_timestamp(0, 0).expect("epoch is a valid timestamp")
}

/// A prompt → command → output unit. The addressable cell and the
/// copy-paste unit of a recorded session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    /// Command submission time (for duration computation; absent for
    /// command-less blocks).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
}

impl Block {
    /// The block's [`Receipt`], when it is a complete command block.
    ///
    /// `last_at` is the timestamp of the block's last event (its output);
    /// duration is `last_at - started_at`. Command-less blocks (raw runs)
    /// yield `None`.
    pub fn receipt(&self, last_at: DateTime<Utc>) -> Option<Receipt> {
        let command = self.command.as_ref()?;
        let started_at = self.started_at?;
        let duration = (last_at - started_at).to_std().unwrap_or(Duration::ZERO);
        Some(Receipt {
            command: command.clone(),
            output: self.output.clone(),
            exit_code: self.exit_code,
            duration,
            cwd: self.cwd.clone(),
        })
    }
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

    /// Append an event, assigning the next monotonic id and the `at`
    /// timestamp.
    ///
    /// The id and timestamp are assigned here — callers never mint them
    /// themselves, which keeps the stream's monotonicity invariant
    /// mechanical.
    pub fn push(&mut self, event: TerminalEvent) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let at = Utc::now();
        let stamped = match event {
            TerminalEvent::Prompt { at: _, cwd, .. } => TerminalEvent::Prompt { id, at, cwd },
            TerminalEvent::Command {
                id: _,
                text,
                started_at,
                at: _,
            } => TerminalEvent::Command {
                id,
                text,
                started_at,
                at,
            },
            TerminalEvent::Output {
                id: _,
                data,
                exit_code,
                at: _,
            } => TerminalEvent::Output {
                id,
                data,
                exit_code,
                at,
            },
            TerminalEvent::Raw { id: _, data, at: _ } => TerminalEvent::Raw { id, data, at },
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
            at: Utc::now(),
        });
        let output_id = self.push(TerminalEvent::Output {
            id: 0,
            data: receipt.output.clone(),
            exit_code: receipt.exit_code,
            at: Utc::now(),
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
                Some(TerminalEvent::Command {
                    id: _cid,
                    text,
                    started_at,
                    ..
                }),
                Some(TerminalEvent::Output {
                    id: oid,
                    data,
                    exit_code,
                    ..
                }),
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
                    started_at: Some(*started_at),
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
                    started_at: None,
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
        assert!(blocks[0].started_at.is_some());
        assert_eq!(blocks[1].command.as_deref(), Some("echo b"));
        assert_eq!(blocks[1].exit_code, Some(1));
        assert_eq!(blocks[1].first_event_id, 3);
    }

    #[test]
    fn raw_events_never_lost_on_grouping() {
        let mut s = SessionStream::new();
        s.push(TerminalEvent::Raw {
            id: 0,
            data: "???".into(),
            at: Utc::now(),
        });
        s.push_receipt(&receipt("echo a", "a", 0));
        let blocks = s.blocks();
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].command.is_none());
        assert_eq!(blocks[0].output, "???");
        assert_eq!(blocks[1].command.as_deref(), Some("echo a"));
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
