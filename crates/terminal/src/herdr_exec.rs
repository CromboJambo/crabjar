//! Structured herdr execution (ADR-005): command → typed [`Receipt`].
//!
//! The segmentation spike's deliverable. Command/output boundaries come
//! from the structured round-trip (`pane run` + sentinel + `wait-output` +
//! `pane read`), not from PTY scraping. Lives in its own module because
//! `herdr.rs` (the backend trait impl) already carries the session
//! mapping; this is a distinct concern with its own protocol knowledge.

use anyhow::Context;

use crate::herdr::HerdrBackend;
use crate::stream::Receipt;

impl HerdrBackend {
    /// The pane's current working directory, when herdr reports one.
    pub async fn pane_cwd(&self, session_name: &str) -> anyhow::Result<Option<String>> {
        let (_, pane_id) = self.session_pane(session_name)?;
        let result = self.run_json(&["pane", "get", &pane_id]).await?;
        // `pane get` nests the pane object under `.pane`:
        // { "result": { "pane": { "cwd": ..., ... }, "type": "pane_info" } }
        Ok(result
            .get("pane")
            .and_then(|p| p.get("cwd"))
            .and_then(|v| v.as_str())
            .map(String::from))
    }

    /// Run a command in the session's pane and return a typed [`Receipt`].
    ///
    /// This is the structured execution path (ADR-005): no PTY scraping.
    ///
    /// 1. `pane run <pane> <echo start-marker>; <command>; echo <sentinel>$?`
    ///    — the command runs in the pane's own shell, then the sentinel
    ///    line carries its exit code.
    /// 2. `pane wait-output --regex ^<sentinel>[0-9]` — blocks until the
    ///    command's sentinel line appears (bounded by `timeout`).
    /// 3. `pane read --source recent` — the output is the text between
    ///    the start marker and the sentinel line.
    ///
    /// The pane's shell still shows the command and its output (the
    /// session stays observable), but the *receipt* comes from the
    /// structured round-trip, not from parsing the prompt.
    ///
    /// Note: the wrapper line is parsed by the pane's shell, so the
    /// command must be a single shell line (pipelines, `&&`, etc. are
    /// fine; multi-line scripts are not).
    pub async fn run_command(
        &self,
        session_name: &str,
        command: &str,
        timeout_ms: u64,
    ) -> anyhow::Result<Receipt> {
        let (_, pane_id) = self.session_pane(session_name)?;
        let cwd = self.pane_cwd(session_name).await?;
        let started = std::time::Instant::now();

        // Unique markers so sequential runs never collide.
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nonce = format!(
            "{}{:x}",
            std::process::id(),
            SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        let start_marker = format!("CRABJAR_START_{nonce}");
        let sentinel = format!("CRABJAR_EXIT_{nonce}");

        // One line, parsed by the pane's shell: start marker, the command
        // verbatim, then the exit-code sentinel.
        let wrapper = format!("echo {start_marker}; {command}; echo {sentinel}$?");

        self.run_json(&["pane", "run", &pane_id, &wrapper])
            .await
            .context("pane run")?;

        // Wait for the sentinel (command completion). Bounded.
        //
        // Must be a regex, not --match: the pane echoes the submitted
        // line, which contains the sentinel followed by `$?` — a plain
        // substring match fires on the echo before the command runs.
        // Requiring a digit after the sentinel matches only the real
        // exit-code line.
        let wait_regex = format!("^{}[0-9]", sentinel);
        let wait = self
            .run_json(&[
                "pane",
                "wait-output",
                &pane_id,
                "--regex",
                &wait_regex,
                "--timeout",
                &timeout_ms.to_string(),
            ])
            .await
            .context("pane wait-output (command did not complete in time)")?;
        if wait.get("type").and_then(|v| v.as_str()) != Some("output_matched") {
            anyhow::bail!("wait-output returned unexpected result: {wait}");
        }

        // Read the pane and slice [start_marker, sentinel].
        // wait-output can match before the whole line range is flushed to
        // the read buffer, so retry the read a few times until both
        // markers are present.
        let mut text = String::new();
        for _ in 0..20 {
            text = self
                .run_raw(&[
                    "pane", "read", &pane_id, "--source", "recent", "--lines", "2000",
                ])
                .await
                .context("pane read")?;
            let has_sentinel = text.contains(&sentinel);
            let has_start = text.contains(&start_marker);
            if has_sentinel && has_start {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        let lines: Vec<&str> = text.lines().collect();
        let end = lines
            .iter()
            .rposition(|l| l.contains(&sentinel))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "sentinel line not found in pane read:\n{}",
                    &text.lines().take(20).collect::<Vec<_>>().join("\n")
                )
            })?;
        let start = lines
            .iter()
            .rposition(|l| l.contains(&start_marker))
            .filter(|&s| s < end)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "start marker line not found in pane read:\n{}",
                    &text.lines().take(20).collect::<Vec<_>>().join("\n")
                )
            })?;

        let output = lines[start + 1..end].join("\n");
        let exit_code = lines[end]
            .split(&sentinel)
            .nth(1)
            .and_then(|rest| rest.parse::<i32>().ok());

        Ok(Receipt {
            command: command.to_string(),
            output,
            exit_code,
            duration: started.elapsed(),
            cwd,
        })
    }
}
