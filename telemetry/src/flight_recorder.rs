use crate::error::{FlightRecorderError, TelemetryError};
use crate::schema::{
    checkpoint_session, init_db, query_flight_records, query_session_checkpoints, query_transcript,
    record_command, record_transcript_line,
};
use rusqlite::Connection;
use sha2::Digest;
use tokio::io::AsyncBufReadExt;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Flight recorder that captures command transcripts, stdout/stderr, exit codes, git diff snapshots, and session checkpoints.
///
/// Every action is treated as an auditable transaction. Not just "here's the final diff," but "here's the chain of custody for how this diff happened."
pub struct FlightRecorder<'a> {
    conn: &'a Connection,
    pub session_id: String,
}

impl<'a> FlightRecorder<'a> {
    pub fn new(conn: &'a Connection, session_id: impl Into<String>) -> Self {
        Self {
            conn,
            session_id: session_id.into(),
        }
    }

    /// Initialize the flight recorder database.
    pub fn init(&self) -> Result<(), FlightRecorderError> {
        init_db(self.conn)
    }

    /// Checkpoint the repo state before work begins.
    pub fn checkpoint_before(
        &self,
        repo_state_hash: &str,
        dirty_tree_count: i32,
    ) -> Result<String, FlightRecorderError> {
        checkpoint_session(
            self.conn,
            &self.session_id,
            repo_state_hash,
            dirty_tree_count,
        )
    }

    /// Run a command with full telemetry capture.
    pub async fn execute_command(
        &self,
        command: &str,
        args: &[String],
        cwd: &str,
        reason: &str,
    ) -> Result<String, FlightRecorderError> {
        let cmd_id = Uuid::new_v4().to_string();

        info!(
            session_id = %self.session_id,
            command = %command,
            args = ?args,
            cwd = %cwd,
            %reason,
            "Flight recorder: executing command"
        );

        let mut child = tokio::process::Command::new(command)
            .args(args)
            .current_dir(cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| FlightRecorderError::CommandCapture(e.to_string()))?;

        let stdout = child.stdout.take().expect("Failed to take stdout");
        let stderr = child.stderr.take().expect("Failed to take stderr");

        let mut stdout_reader = tokio::io::BufReader::new(stdout).lines();
        let mut stderr_reader = tokio::io::BufReader::new(stderr).lines();

        let mut stdout_lines = Vec::new();
        let mut stderr_lines = Vec::new();

        loop {
            tokio::select! {
                line = stdout_reader.next_line() => {
                    match line {
                        Ok(Some(l)) => {
                            stdout_lines.push(l.clone());
                            record_transcript_line(self.conn, &self.session_id, &cmd_id, stdout_lines.len() as i32, &l, "")?;
                        }
                        Ok(None) => {}, // EOF for stdout
                        Err(e) => {
                            warn!(session_id = %self.session_id, %e, "Flight recorder: stdout read error");
                            break;
                        }
                    }
                }
                line = stderr_reader.next_line() => {
                    match line {
                        Ok(Some(l)) => {
                            stderr_lines.push(l.clone());
                            record_transcript_line(self.conn, &self.session_id, &cmd_id, stderr_lines.len() as i32, "", &l)?;
                        }
                        Ok(None) => {}, // EOF for stderr
                        Err(e) => {
                            warn!(session_id = %self.session_id, %e, "Flight recorder: stderr read error");
                            break;
                        }
                    }
                }
                status = child.wait() => {
                    let exit_code = status.map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);

                    let stdout_hash = sha256(&stdout_lines.join("\n"));
                    let stderr_hash = sha256(&stderr_lines.join("\n"));

                    record_command(
                        self.conn,
                        &self.session_id,
                        command,
                        cwd,
                        args,
                        exit_code,
                        &stdout_hash,
                        &stderr_hash,
                        stdout_lines.len(),
                        stderr_lines.len(),
                        0,
                        0,
                        "",
                        reason,
                        "agent",
                    )?;

                    info!(
                        session_id = %self.session_id,
                        command = %command,
                        exit_code = exit_code,
                        stdout_len = stdout_lines.len(),
                        stderr_len = stderr_lines.len(),
                        "Flight recorder: command completed"
                    );

                    break;
                }
            }
        }

        Ok(cmd_id)
    }

    /// Checkpoint the repo state after work completes.
    pub fn checkpoint_after(
        &self,
        repo_state_hash: &str,
        dirty_tree_count: i32,
    ) -> Result<String, FlightRecorderError> {
        checkpoint_session(
            self.conn,
            &self.session_id,
            repo_state_hash,
            dirty_tree_count,
        )
    }

    /// Query flight records for a session.
    pub fn query_records(
        &self,
        limit: usize,
    ) -> Result<Vec<crate::schema::FlightRecordRow>, FlightRecorderError> {
        query_flight_records(self.conn, &self.session_id, limit)
    }

    /// Query session checkpoints.
    pub fn query_checkpoints(
        &self,
        limit: usize,
    ) -> Result<Vec<crate::schema::CheckpointRow>, FlightRecorderError> {
        query_session_checkpoints(self.conn, &self.session_id, limit)
    }

    /// Query transcript lines for a specific command.
    pub fn query_transcript(
        &self,
        command_id: &str,
        limit: usize,
    ) -> Result<Vec<crate::schema::TranscriptRow>, FlightRecorderError> {
        query_transcript(self.conn, &self.session_id, command_id, limit)
    }

    /// Capture git dirty tree count before a command.
    pub async fn capture_git_dirty(&self, cwd: &str) -> Result<i32, TelemetryError> {
        let git_status = tokio::process::Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(cwd)
            .output()
            .await
            .map_err(|e| TelemetryError::GitDiff(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&git_status.stdout);
        let count = stdout.lines().count() as i32;

        debug!(
            session_id = %self.session_id,
            cwd = %cwd,
            dirty_count = count,
            "Flight recorder: git dirty tree count"
        );

        Ok(count)
    }

    /// Capture git diff hash after a command.
    pub async fn capture_git_diff(&self, cwd: &str) -> Result<String, TelemetryError> {
        let git_diff = tokio::process::Command::new("git")
            .args(["diff", "--stat"])
            .current_dir(cwd)
            .output()
            .await
            .map_err(|e| TelemetryError::GitDiff(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&git_diff.stdout);
        let hash = sha256(&stdout);

        debug!(
            session_id = %self.session_id,
            diff_count = stdout.lines().count(),
            "Flight recorder: git diff captured"
        );

        Ok(hash)
    }
}

/// Simple SHA-256 hash for telemetry purposes.
fn sha256(data: &str) -> String {
    if data.is_empty() {
        return String::new();
    }

    let mut hasher = sha2::Sha256::new();
    hasher.update(data.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_flight_recorder_init() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("flight.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        let recorder = FlightRecorder::new(&conn, "test-session");
        recorder.init().unwrap();

        let rows = recorder.query_records(10).unwrap();
        assert_eq!(rows.len(), 0);
    }

    #[test]
    fn test_checkpoint_before_and_after() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("flight.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        let recorder = FlightRecorder::new(&conn, "test-session");
        recorder.init().unwrap();

        let before_id = recorder.checkpoint_before("before_hash", 0).unwrap();
        let after_id = recorder.checkpoint_after("after_hash", 5).unwrap();
        assert!(!before_id.is_empty());
        assert!(!after_id.is_empty());

        let checkpoints = recorder.query_checkpoints(10).unwrap();
        assert_eq!(checkpoints.len(), 2);
        assert_eq!(checkpoints[0].repo_state_hash, "before_hash");
        assert_eq!(checkpoints[1].repo_state_hash, "after_hash");
    }

    #[tokio::test]
    async fn test_capture_git_dirty() {
        let dir = tempdir().unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("flight.db")).unwrap();

        let recorder = FlightRecorder::new(&conn, "test-session");
        recorder.init().unwrap();

        let dirty = recorder
            .capture_git_dirty(dir.path().to_string_lossy().as_ref())
            .await
            .unwrap();
        assert_eq!(dirty, 0);
    }
}
