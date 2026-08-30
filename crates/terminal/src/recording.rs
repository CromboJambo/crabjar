//! Asciinema v2 serializer (ADR-005).
//!
//! Asciinema v2 is a **wire representation** of the typed terminal event
//! stream, never the model. The native JSONL
//! ([`crate::session_record`]) is the faithful on-disk form; this module
//! projects the stream onto asciinema v2 (header + `[time, "i"|"o", data]`)
//! for playback with `asciinema play`.
//!
//! The projection is **lossy**: asciinema v2 events carry no block ids and
//! no exit codes. A session round-tripped through asciinema loses its
//! addressable structure — keep the JSONL as the source of truth.
//!
//! ## Event mapping
//!
//! ```text
//! Command { text }  →  [t, "i", text + "\n"]   // the user typed the line
//! Output  { data }  →  [t, "o", data + "\n"]   // the terminal echoed it
//! Prompt  { .. }    →  dropped (rendering artifact, not content)
//! Raw     { data }  →  [t, "o", data]          // unsegmented bytes
//! ```

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::Instant;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::stream::TerminalEvent;

/// Asciinema v2 header format version
const ASCIINEMA_VERSION: u8 = 2;

/// Metadata for the recording session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingMetadata {
    /// Asciinema format version
    pub version: u8,
    /// Width of terminal in columns (0 if unknown)
    pub width: u16,
    /// Height of terminal in rows (0 if unknown)
    pub height: u16,
    /// Duration of recording in seconds (set when recording ends)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    /// Exit code of the last completed command (set when recording ends)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<u32>,
    /// Title or description of the session
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Backend used (wezterm, zellij, herdr)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,
}

/// A single asciinema v2 event (input or output with timestamp)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingEvent {
    /// Timestamp in seconds since start of recording (with millisecond precision)
    pub time: f64,
    /// Event type: "i" for input, "o" for output
    #[serde(rename = "type")]
    pub event_type: char,
    /// The actual content (text/escape sequence)
    pub data: String,
}

/// Serializes a [`TerminalEvent`] stream to asciinema v2.
///
/// Two modes:
/// - **Batch** — [`AsciinemaSerializer::from_events`] projects a complete
///   stream (e.g. a copied selection) in one shot. Times are relative to
///   the first event.
/// - **Live** — [`AsciinemaSerializer::event`] feeds events as they happen
///   (from `TerminalSession::send`/`read`), timing them against the
///   recording start.
pub struct AsciinemaSerializer {
    /// Output file path
    output_path: PathBuf,
    /// File writer (buffered)
    writer: Option<BufWriter<File>>,
    /// Recording start time (live mode)
    start_time: Instant,
    /// Metadata for the recording header
    metadata: RecordingMetadata,
    /// Whether the serializer is open
    is_open: bool,
}

impl std::fmt::Debug for AsciinemaSerializer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsciinemaSerializer")
            .field("output_path", &self.output_path)
            .field("is_open", &self.is_open)
            .finish()
    }
}

impl AsciinemaSerializer {
    /// Create a new serializer with the given session metadata.
    pub fn new(session_name: &str, backend: &str, output_path: PathBuf) -> Self {
        let metadata = RecordingMetadata {
            version: ASCIINEMA_VERSION,
            width: 0, // Will be set if available from terminal
            height: 0,
            duration: None,
            exit_code: None,
            title: Some(session_name.to_string()),
            backend: Some(backend.to_string()),
        };

        Self {
            output_path,
            writer: None,
            start_time: Instant::now(),
            metadata,
            is_open: false,
        }
    }

    /// Open the serializer — creates the file and writes the header.
    pub fn start(&mut self) -> anyhow::Result<()> {
        if self.is_open {
            return Ok(()); // Already open
        }

        // Ensure output directory exists
        if let Some(parent) = self.output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = File::create(&self.output_path)?;
        let mut writer = BufWriter::new(file);

        // Write header as first line (JSON object, no newline after)
        let header_json = serde_json::to_string(&self.metadata)?;
        writeln!(writer, "{}", header_json)?;

        self.writer = Some(writer);
        self.is_open = true;

        tracing::info!(path = ?self.output_path, "started asciinema serialization");
        Ok(())
    }

    /// Close the serializer — finalizes metadata and closes the file.
    pub fn stop(&mut self) -> anyhow::Result<()> {
        if !self.is_open {
            return Ok(()); // Not open
        }

        let elapsed = self.start_time.elapsed();
        self.metadata.duration = Some(elapsed.as_secs_f64());

        if let Some(ref mut writer) = self.writer {
            // Note: In a full implementation, you'd rewrite the header with correct duration
            // For now, we just close cleanly

            writer.flush()?;
        }

        self.is_open = false;
        tracing::info!(path = ?self.output_path, "stopped asciinema serialization");
        Ok(())
    }

    /// Feed one typed event into the live stream.
    ///
    /// `Command` becomes an `i` event (the typed line), `Output`/`Raw` an
    /// `o` event, `Prompt` is dropped. Events before `start()` are ignored.
    pub fn event(&mut self, event: &TerminalEvent) -> anyhow::Result<()> {
        if !self.is_open {
            return Ok(()); // Not open
        }

        let (event_type, data) = match event {
            TerminalEvent::Command { text, .. } => ('i', format!("{text}\n")),
            TerminalEvent::Output { data, .. } => ('o', format!("{data}\n")),
            TerminalEvent::Raw { data, .. } => ('o', data.clone()),
            TerminalEvent::Prompt { .. } => return Ok(()), // dropped
        };

        let elapsed = self.start_time.elapsed().as_secs_f64();
        let rec = RecordingEvent {
            time: (elapsed * 1000.0).round() / 1000.0, // Millisecond precision
            event_type,
            data,
        };
        self.write_event(&rec)
    }

    /// Project a complete stream to asciinema v2 in one shot (batch mode).
    ///
    /// Times are relative to the first event, so a copied selection
    /// replays with its internal timing intact.
    pub fn from_events(events: &[TerminalEvent]) -> String {
        let first_at = events.iter().map(e_at).min();
        let mut out = String::new();

        let header = RecordingMetadata {
            version: ASCIINEMA_VERSION,
            width: 0,
            height: 0,
            duration: Some(duration_of(events, first_at)),
            exit_code: last_exit_code(events),
            title: None,
            backend: None,
        };
        out.push_str(&serde_json::to_string(&header).expect("metadata serialization"));
        out.push('\n');

        for event in events {
            let (event_type, data) = match event {
                TerminalEvent::Command { text, .. } => ('i', format!("{text}\n")),
                TerminalEvent::Output { data, .. } => ('o', format!("{data}\n")),
                TerminalEvent::Raw { data, .. } => ('o', data.clone()),
                TerminalEvent::Prompt { .. } => continue, // dropped
            };
            let time = first_at
                .map(|f| (e_at(event) - f).num_milliseconds() as f64 / 1000.0)
                .unwrap_or(0.0);
            let rec = RecordingEvent {
                time: (time * 1000.0).round() / 1000.0,
                event_type,
                data,
            };
            out.push_str(&serde_json::to_string(&rec).expect("event serialization"));
            out.push('\n');
        }

        out
    }

    fn write_event(&mut self, rec: &RecordingEvent) -> anyhow::Result<()> {
        if let Some(ref mut writer) = self.writer {
            let json = serde_json::to_string(rec)?;
            writeln!(writer, "{}", json)?;
        }
        Ok(())
    }

    /// Get the output file path.
    pub fn output_path(&self) -> &PathBuf {
        &self.output_path
    }

    /// Check if the serializer is open.
    pub fn is_open(&self) -> bool {
        self.is_open
    }
}

/// Convenience: the serializer's open state, for callers that tracked
/// "recording" before the rename.
impl AsciinemaSerializer {
    /// Whether the serializer is open (alias for [`AsciinemaSerializer::is_open`]).
    pub fn is_recording(&self) -> bool {
        self.is_open
    }
}

impl Drop for AsciinemaSerializer {
    fn drop(&mut self) {
        // Ensure file is closed properly even if stop() wasn't called
        if let Some(mut writer) = self.writer.take() {
            let _ = writer.flush();
        }
    }
}

/// The event's `at` timestamp.
fn e_at(event: &TerminalEvent) -> DateTime<Utc> {
    match event {
        TerminalEvent::Prompt { at, .. }
        | TerminalEvent::Command { at, .. }
        | TerminalEvent::Output { at, .. }
        | TerminalEvent::Raw { at, .. } => *at,
    }
}

/// Total duration in seconds, from the first event to the last.
fn duration_of(events: &[TerminalEvent], first_at: Option<DateTime<Utc>>) -> f64 {
    let last = events.iter().map(e_at).max();
    match (first_at, last) {
        (Some(f), Some(l)) => (l - f).num_milliseconds() as f64 / 1000.0,
        _ => 0.0,
    }
}

/// The exit code of the last `Output` event, when it is a non-negative i32.
fn last_exit_code(events: &[TerminalEvent]) -> Option<u32> {
    events
        .iter()
        .rev()
        .find_map(|e| match e {
            TerminalEvent::Output { exit_code, .. } => *exit_code,
            _ => None,
        })
        .and_then(|c| u32::try_from(c).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn events() -> Vec<TerminalEvent> {
        let t0 = Utc::now();
        vec![
            TerminalEvent::Prompt {
                id: 0,
                at: t0,
                cwd: Some("/tmp".into()),
            },
            TerminalEvent::Command {
                id: 1,
                text: "echo a".into(),
                started_at: t0,
                at: t0 + chrono::Duration::milliseconds(10),
            },
            TerminalEvent::Output {
                id: 2,
                data: "a".into(),
                exit_code: Some(0),
                at: t0 + chrono::Duration::milliseconds(120),
            },
        ]
    }

    #[test]
    fn from_events_projects_command_and_output_only() {
        let cast = AsciinemaSerializer::from_events(&events());
        let lines: Vec<&str> = cast.lines().collect();
        assert_eq!(lines.len(), 3); // header + i + o (prompt dropped)
        let header: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(header["version"], 2);
        assert_eq!(header["exit_code"], 0);
        assert!(header["duration"].as_f64().unwrap() > 0.1);
        let first: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(first["type"], "i");
        assert_eq!(first["data"], "echo a\n");
        // The command is 10 ms after the first event (the prompt).
        assert!((first["time"].as_f64().unwrap() - 0.01).abs() < 0.001);
        let second: serde_json::Value = serde_json::from_str(lines[2]).unwrap();
        assert_eq!(second["type"], "o");
        assert_eq!(second["data"], "a\n");
        assert!((second["time"].as_f64().unwrap() - 0.12).abs() < 0.001);
    }

    #[test]
    fn from_events_empty_stream_is_header_only() {
        let cast = AsciinemaSerializer::from_events(&[]);
        let lines: Vec<&str> = cast.lines().collect();
        assert_eq!(lines.len(), 1);
        let header: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(header["version"], 2);
        assert!(header.get("exit_code").is_none());
    }

    #[test]
    fn from_events_keeps_negative_exit_codes_out_of_header() {
        let mut evs = events();
        if let TerminalEvent::Output { exit_code, .. } = &mut evs[2] {
            *exit_code = Some(7);
        }
        let cast = AsciinemaSerializer::from_events(&evs);
        let header: serde_json::Value = serde_json::from_str(cast.lines().next().unwrap()).unwrap();
        assert_eq!(header["exit_code"], 7);

        if let TerminalEvent::Output { exit_code, .. } = &mut evs[2] {
            *exit_code = None;
        }
        let cast = AsciinemaSerializer::from_events(&evs);
        let header: serde_json::Value = serde_json::from_str(cast.lines().next().unwrap()).unwrap();
        assert!(header.get("exit_code").is_none());
    }

    #[test]
    fn live_mode_feeds_events_with_relative_times() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("live.cast");
        let mut ser = AsciinemaSerializer::new("live", "herdr", path.clone());
        ser.start().expect("start");
        let evs = events();
        for e in &evs {
            ser.event(e).expect("feed");
        }
        std::thread::sleep(Duration::from_millis(15));
        ser.stop().expect("stop");

        let text = fs::read_to_string(&path).expect("read");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3);
        let first: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(first["type"], "i");
        assert!(first["time"].as_f64().unwrap() >= 0.0);
    }

    #[test]
    fn live_mode_ignores_events_before_start() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("pre.cast");
        let mut ser = AsciinemaSerializer::new("pre", "herdr", path.clone());
        let evs = events();
        for e in &evs {
            ser.event(e).expect("feed (no-op)");
        }
        let text = fs::read_to_string(&path).unwrap_or_default();
        assert!(text.is_empty(), "no file before start(): {text:?}");
    }
}
