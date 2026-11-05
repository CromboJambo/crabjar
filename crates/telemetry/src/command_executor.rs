use crate::error::{FlightRecorderError, TelemetryError};
use crate::flight_recorder::FlightRecorder;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Structured command executor that pipes stdout/stderr into session logs with deterministic replay metadata.
///
/// Every command run produces: command, cwd, args, timestamps, stdout/stderr capture, exit codes, git diff snapshots.
pub struct StructuredCommandExecutor<'a> {
    flight_recorder: FlightRecorder<'a>,
}

impl<'a> StructuredCommandExecutor<'a> {
    pub fn new(flight_recorder: FlightRecorder<'a>) -> Self {
        Self { flight_recorder }
    }

    /// Execute a command with full telemetry capture.
    pub async fn run(
        &mut self,
        command: &str,
        args: &[String],
        cwd: &str,
        reason: &str,
    ) -> Result<CommandOutcome, TelemetryError> {
        let cmd_id = self
            .flight_recorder
            .execute_command(command, args, cwd, reason)
            .await?;

        let exit_code = self.flight_recorder.query_records(1)?;
        let exit_code = exit_code.first().map(|r| r.exit_code).unwrap_or(-1);

        let git_dirty = self.flight_recorder.capture_git_dirty(cwd).await?;
        let git_diff = self.flight_recorder.capture_git_diff(cwd).await?;

        info!(
            session_id = %self.flight_recorder.session_id,
            command = %command,
            exit_code = exit_code,
            cmd_id = %cmd_id,
            "Structured command executor: command completed with telemetry"
        );

        Ok(CommandOutcome {
            cmd_id,
            command: command.to_string(),
            args: args.to_vec(),
            cwd: cwd.to_string(),
            exit_code,
            git_dirty,
            git_diff_hash: git_diff,
            reason: reason.to_string(),
            source: "agent".to_string(),
        })
    }

    /// Execute a command with dry-run mode (no execution, only capture).
    pub async fn dry_run(
        &self,
        command: &str,
        args: &[String],
        cwd: &str,
        reason: &str,
    ) -> Result<CommandOutcome, TelemetryError> {
        let git_dirty = self.flight_recorder.capture_git_dirty(cwd).await?;
        let git_diff = self.flight_recorder.capture_git_diff(cwd).await?;

        debug!(
            session_id = %self.flight_recorder.session_id,
            command = %command,
            args = ?args,
            "Structured command executor: dry-run capture"
        );

        Ok(CommandOutcome {
            cmd_id: uuid::Uuid::new_v4().to_string(),
            command: command.to_string(),
            args: args.to_vec(),
            cwd: cwd.to_string(),
            exit_code: -1,
            git_dirty,
            git_diff_hash: git_diff,
            reason: reason.to_string(),
            source: "dry-run".to_string(),
        })
    }

    /// Query all command outcomes for a session.
    pub fn query_outcomes(
        &self,
        limit: usize,
    ) -> Result<Vec<crate::schema::FlightRecordRow>, FlightRecorderError> {
        self.flight_recorder.query_records(limit)
    }
}

/// Outcome of a structured command execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandOutcome {
    pub cmd_id: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub exit_code: i32,
    pub git_dirty: i32,
    pub git_diff_hash: String,
    pub reason: String,
    pub source: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_structured_executor_dry_run() {
        let dir = tempdir().unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("flight.db")).unwrap();

        let recorder = FlightRecorder::new(&conn, "test-session");
        recorder.init().unwrap();

        let executor = StructuredCommandExecutor::new(recorder);

        let outcome = executor
            .dry_run(
                "cargo check",
                &["--workspace".to_string()],
                dir.path().to_string_lossy().as_ref(),
                "dry run check",
            )
            .await
            .unwrap();

        assert_eq!(outcome.exit_code, -1);
        assert_eq!(outcome.source, "dry-run");
    }

    #[tokio::test]
    async fn test_structured_executor_run_echo() {
        let dir = tempdir().unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("flight.db")).unwrap();

        let recorder = FlightRecorder::new(&conn, "test-session-run");
        recorder.init().unwrap();

        let mut executor = StructuredCommandExecutor::new(recorder);

        let outcome = executor
            .run(
                "echo",
                &["hello".to_string()],
                dir.path().to_string_lossy().as_ref(),
                "test run",
            )
            .await
            .unwrap();

        assert_eq!(outcome.exit_code, 0);
        assert_eq!(outcome.command, "echo");
        assert_eq!(outcome.source, "agent");
    }

    #[tokio::test]
    async fn test_structured_executor_run_fails() {
        let dir = tempdir().unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("flight.db")).unwrap();

        let recorder = FlightRecorder::new(&conn, "test-session-fail");
        recorder.init().unwrap();

        let mut executor = StructuredCommandExecutor::new(recorder);

        let outcome = executor
            .run(
                "nonexistent_binary_xyz",
                &[],
                dir.path().to_string_lossy().as_ref(),
                "test fail",
            )
            .await;

        assert!(outcome.is_err());
    }

    #[test]
    fn test_query_outcomes_empty() {
        let dir = tempdir().unwrap();
        let conn = rusqlite::Connection::open(dir.path().join("flight.db")).unwrap();

        let recorder = FlightRecorder::new(&conn, "test-session-q");
        recorder.init().unwrap();

        let executor = StructuredCommandExecutor::new(recorder);
        let outcomes = executor.query_outcomes(10).unwrap();
        assert!(outcomes.is_empty());
    }
}
