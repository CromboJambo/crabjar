//! Asciinema v2 terminal session recording.
//!
//! Terminal session recording in asciinema v2 format.
//!
//! Provides support for recording terminal sessions to the asciinema v2 format,
//! which is a widely-supported JSON-based format with timestamps and escape sequences.

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::Instant;

use serde::{Deserialize, Serialize};

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
    /// Exit code of the recorded command (set when recording ends)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<u32>,
    /// Title or description of the session
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Backend used (wezterm, zellij)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,
}

/// A single recording event (input or output with timestamp)
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

/// Terminal session recorder — writes asciinema v2 format.
#[derive(Debug)]
///
/// Manages the lifecycle of a terminal recording, including header writing,
/// event buffering, and proper file closing with metadata completion.
pub struct AsciinemaRecorder {
    /// Output file path
    output_path: PathBuf,
    /// File writer (buffered)
    writer: Option<BufWriter<File>>,
    /// Recording start time
    start_time: Instant,
    /// Metadata for the recording header
    metadata: RecordingMetadata,
    /// Whether recording has been started
    is_recording: bool,
}

impl AsciinemaRecorder {
    /// Create a new recorder with the given session metadata.
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
            is_recording: false,
        }
    }

    /// Start the recording — creates and writes header to file.
    pub fn start(&mut self) -> anyhow::Result<()> {
        if self.is_recording {
            return Ok(()); // Already started
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
        self.is_recording = true;

        tracing::info!(path = ?self.output_path, "started asciinema recording");
        Ok(())
    }

    /// Stop the recording — finalizes metadata and closes file.
    pub fn stop(&mut self) -> anyhow::Result<()> {
        if !self.is_recording {
            return Ok(()); // Not recording
        }

        let elapsed = self.start_time.elapsed();
        self.metadata.duration = Some(elapsed.as_secs_f64());

        if let Some(ref mut writer) = self.writer {
            // Note: In a full implementation, you'd rewrite the header with correct duration
            // For now, we just close cleanly

            writer.flush()?;
        }

        self.is_recording = false;
        tracing::info!(path = ?self.output_path, "stopped asciinema recording");
        Ok(())
    }

    /// Record an input event (user typing or command sent).
    pub fn record_input(&mut self, data: &str) -> anyhow::Result<()> {
        if !self.is_recording {
            return Ok(()); // Not recording
        }

        let elapsed = self.start_time.elapsed().as_secs_f64();

        let event = RecordingEvent {
            time: (elapsed * 1000.0).round() / 1000.0, // Millisecond precision
            event_type: 'i',
            data: data.to_string(),
        };

        if let Some(ref mut writer) = self.writer {
            let json = serde_json::to_string(&event)?;
            writeln!(writer, "{}", json)?;
        }

        Ok(())
    }

    /// Record an output event (terminal response).
    pub fn record_output(&mut self, data: &str) -> anyhow::Result<()> {
        if !self.is_recording {
            return Ok(()); // Not recording
        }

        let elapsed = self.start_time.elapsed().as_secs_f64();

        let event = RecordingEvent {
            time: (elapsed * 1000.0).round() / 1000.0,
            event_type: 'o',
            data: data.to_string(),
        };

        if let Some(ref mut writer) = self.writer {
            let json = serde_json::to_string(&event)?;
            writeln!(writer, "{}", json)?;
        }

        Ok(())
    }

    /// Get the output file path.
    pub fn output_path(&self) -> &PathBuf {
        &self.output_path
    }

    /// Check if recording is active.
    pub fn is_recording(&self) -> bool {
        self.is_recording
    }
}

impl Drop for AsciinemaRecorder {
    fn drop(&mut self) {
        // Ensure file is closed properly even if stop() wasn't called
        if let Some(mut writer) = self.writer.take() {
            let _ = writer.flush();
        }
    }
}
