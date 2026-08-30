//! Type-safe copy-paste for recorded sessions (ADR-005).
//!
//! Copy = select an event id range or a block. Paste = serialize the
//! selection to a typed target. The selection is the unit that crosses the
//! glass: it is plain data (a slice of [`TerminalEvent`]s) that any wire
//! serializer can consume.
//!
//! ## Targets
//!
//! - **Native JSONL** — the faithful form: the selection re-serializes as a
//!   standalone [`SessionRecord`] (a sub-session).
//! - **Asciinema v2** — the lossy projection: the selection re-serializes as
//!   a `.cast` file (see [`crate::recording`]). Block ids and exit codes are
//!   dropped; the on-disk JSONL keeps them.
//!
//! A completed block also yields a [`Receipt`] — the input shape the
//! deterministic task scorers consume.

use crate::recording::AsciinemaSerializer;
use crate::session_record::SessionRecord;
use crate::stream::{Block, Receipt, STREAM_VERSION, SessionStream, TerminalEvent};
use std::time::Duration;

/// A copied selection from a recorded session.
///
/// The selection is defined by event ids (the stream's addressing scheme);
/// the events themselves are carried so the selection is self-contained and
/// can be pasted without the source stream.
#[derive(Debug, Clone, PartialEq)]
pub struct Selection {
    /// The selected events, in stream order.
    pub events: Vec<TerminalEvent>,
}

impl Selection {
    /// The first and last event ids (empty selection: `(0, 0)`).
    pub fn id_range(&self) -> (u64, u64) {
        match (self.events.first(), self.events.last()) {
            (Some(first), Some(last)) => (first.id(), last.id()),
            _ => (0, 0),
        }
    }

    /// Paste to native JSONL (the faithful form): a standalone
    /// [`SessionRecord`] for the selection.
    pub fn paste_jsonl(&self, session: &str, backend: &str) -> SessionRecord {
        SessionRecord {
            version: STREAM_VERSION,
            session: session.to_string(),
            backend: backend.to_string(),
            events: self.events.clone(),
        }
    }

    /// Paste to asciinema v2 (the lossy projection): a `.cast` string.
    pub fn paste_asciinema(&self) -> String {
        AsciinemaSerializer::from_events(&self.events)
    }

    /// The receipts in the selection: one per complete
    /// prompt → command → output triple.
    ///
    /// Duration is derived from event timestamps (command → output); a
    /// selection with epoch-defaulted timestamps (v1 records) yields zero
    /// duration.
    pub fn receipts(&self) -> Vec<Receipt> {
        let mut receipts = Vec::new();
        let mut i = 0;
        while i + 3 <= self.events.len() {
            if let (
                TerminalEvent::Prompt { cwd, .. },
                TerminalEvent::Command {
                    started_at, text, ..
                },
                TerminalEvent::Output {
                    data,
                    exit_code,
                    at,
                    ..
                },
            ) = (&self.events[i], &self.events[i + 1], &self.events[i + 2])
            {
                let duration = (*at - *started_at).to_std().unwrap_or(Duration::ZERO);
                receipts.push(Receipt {
                    command: text.clone(),
                    output: data.clone(),
                    exit_code: *exit_code,
                    duration,
                    cwd: cwd.clone(),
                });
                i += 3;
            } else {
                i += 1;
            }
        }
        receipts
    }
}

/// Copy a contiguous range of event ids (inclusive) from a stream.
///
/// Returns an empty selection when the range is empty or out of bounds.
pub fn copy_event_range(stream: &SessionStream, first_id: u64, last_id: u64) -> Selection {
    if first_id > last_id {
        return Selection { events: vec![] };
    }
    Selection {
        events: stream
            .events()
            .iter()
            .filter(|e| e.id() >= first_id && e.id() <= last_id)
            .cloned()
            .collect(),
    }
}

/// Copy a block (the addressable cell) from a stream.
///
/// Returns `None` when no block in the stream matches `block.first_event_id`.
pub fn copy_block(stream: &SessionStream, block: &Block) -> Option<Selection> {
    let events = stream.events();
    let start = events.iter().position(|e| e.id() == block.first_event_id)?;
    let end = events.iter().position(|e| e.id() == block.last_event_id)?;
    if end < start {
        return None;
    }
    Some(Selection {
        events: events[start..=end].to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::SessionStream as S;
    use chrono::Utc;
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

    fn two_command_stream() -> SessionStream {
        let mut s = S::new();
        s.push_receipt(&receipt("echo a", "a", 0));
        s.push_receipt(&receipt("echo b", "b", 1));
        s
    }

    #[test]
    fn copy_event_range_selects_inclusive_range() {
        let s = two_command_stream();
        let sel = copy_event_range(&s, 1, 2); // command + output of #1
        assert_eq!(sel.events.len(), 2);
        assert_eq!(sel.id_range(), (1, 2));
    }

    #[test]
    fn copy_event_range_rejects_inverted_range() {
        let s = two_command_stream();
        assert!(copy_event_range(&s, 5, 1).events.is_empty());
        assert!(copy_event_range(&s, 100, 200).events.is_empty());
    }

    #[test]
    fn copy_block_returns_exact_triple() {
        let s = two_command_stream();
        let blocks = s.blocks();
        let sel = copy_block(&s, &blocks[1]).expect("block exists");
        assert_eq!(sel.events.len(), 3);
        assert_eq!(sel.id_range(), (3, 5));
    }

    #[test]
    fn paste_jsonl_round_trips_selection() {
        let s = two_command_stream();
        let sel = copy_event_range(&s, 0, 2);
        let record = sel.paste_jsonl("sub", "herdr");
        let parsed = SessionRecord::from_jsonl(&record.to_jsonl()).expect("parse");
        assert_eq!(parsed.events, sel.events);
        assert_eq!(parsed.session, "sub");
    }

    #[test]
    fn paste_asciinema_projects_selection() {
        let s = two_command_stream();
        let sel = copy_event_range(&s, 0, 2);
        let cast = sel.paste_asciinema();
        let lines: Vec<&str> = cast.lines().collect();
        assert_eq!(lines.len(), 3); // header + command + output
        assert!(lines[0].contains("\"version\":2"));
        assert!(lines[1].contains("\"type\":\"i\""));
        assert!(lines[1].contains("echo a"));
        assert!(lines[2].contains("\"type\":\"o\""));
        assert!(lines[2].contains("a"));
    }

    #[test]
    fn selection_receipts_match_source_receipts() {
        let s = two_command_stream();
        let sel = copy_event_range(&s, 0, 5);
        let receipts = sel.receipts();
        assert_eq!(receipts.len(), 2);
        assert_eq!(receipts[0].command, "echo a");
        assert_eq!(receipts[0].exit_code, Some(0));
        assert_eq!(receipts[1].command, "echo b");
        assert_eq!(receipts[1].exit_code, Some(1));
    }

    #[test]
    fn selection_with_raw_events_yields_no_receipts() {
        let mut s = S::new();
        s.push(TerminalEvent::Raw {
            id: 0,
            data: "junk".into(),
            at: Utc::now(),
        });
        let sel = copy_event_range(&s, 0, 0);
        assert!(sel.receipts().is_empty());
    }
}
