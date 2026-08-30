//! Deterministic task scorers (ADR-005).
//!
//! Verifiers run against a [`Receipt`] — the typed output of a recorded
//! command — instead of regex-scraping a raw log. The four verifiers
//! (README parity item): `exit_code`, `file_exists`, `regex_match`,
//! `json_path`.
//!
//! A verifier is a pure function of `(receipt, expectation)` →
//! [`VerifierResult`]. Nothing here touches the filesystem except
//! `file_exists` (which is the expectation, not a side effect of scoring),
//! and nothing here knows where the receipt came from — herdr, wezterm,
//! a replayed JSONL record. That is the payoff: the same scorer runs
//! against a live receipt and a receipt reconstructed from a recording.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::stream::Receipt;

/// The outcome of one verifier run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerifierResult {
    /// The verifier that ran (exit_code, file_exists, regex_match, json_path).
    pub verifier: String,
    /// Whether the expectation held.
    pub passed: bool,
    /// A one-line explanation (the observed value, the match, the path…).
    pub detail: String,
}

/// Expectation for the `exit_code` verifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExitCodeExpectation {
    /// The expected exit code.
    pub code: i32,
}

/// Expectation for the `file_exists` verifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileExistsExpectation {
    /// The path that must exist (relative paths resolve against `cwd`).
    pub path: String,
}

/// Expectation for the `regex_match` verifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegexMatchExpectation {
    /// The regex to search for in the output.
    pub pattern: String,
}

/// Expectation for the `json_path` verifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonPathExpectation {
    /// Dot-separated path into the output's JSON (e.g. `result.status`).
    /// The output must parse as JSON; the path must resolve to a value.
    pub path: String,
    /// The expected value at the path (compared as JSON).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
}

/// Check the receipt's exit code against the expectation.
///
/// A receipt with no exit code (the backend did not report one) fails —
/// absence of evidence is not evidence of the expected code.
pub fn exit_code(receipt: &Receipt, expected: &ExitCodeExpectation) -> VerifierResult {
    let passed = receipt.exit_code == Some(expected.code);
    let detail = match receipt.exit_code {
        Some(actual) => format!("exit_code {actual} (expected {})", expected.code),
        None => format!("no exit code reported (expected {})", expected.code),
    };
    VerifierResult {
        verifier: "exit_code".into(),
        passed,
        detail,
    }
}

/// Check that a file exists. Relative paths resolve against the receipt's
/// `cwd`; a receipt without `cwd` only accepts absolute paths.
pub fn file_exists(receipt: &Receipt, expected: &FileExistsExpectation) -> VerifierResult {
    let path = resolve_path(receipt, &expected.path);
    let path = match &path {
        Some(p) => p.clone(),
        None => {
            return VerifierResult {
                verifier: "file_exists".into(),
                passed: false,
                detail: format!("relative path {:?} but receipt has no cwd", expected.path),
            };
        }
    };
    let passed = Path::new(&path).exists();
    VerifierResult {
        verifier: "file_exists".into(),
        passed,
        detail: format!("{} {}", if passed { "exists" } else { "missing" }, path),
    }
}

/// Check that the output matches a regex.
pub fn regex_match(receipt: &Receipt, expected: &RegexMatchExpectation) -> VerifierResult {
    let compiled = match regex::Regex::new(&expected.pattern) {
        Ok(r) => r,
        Err(e) => {
            return VerifierResult {
                verifier: "regex_match".into(),
                passed: false,
                detail: format!("invalid regex {:?}: {e}", expected.pattern),
            };
        }
    };
    let passed = compiled.is_match(&receipt.output);
    let detail = if passed {
        format!(
            "matched {:?} at line {}",
            expected.pattern,
            compiled
                .find(&receipt.output)
                .map(|m| receipt.output[..m.start()].lines().count())
                .unwrap_or(0)
                + 1
        )
    } else {
        format!("no match for {:?}", expected.pattern)
    };
    VerifierResult {
        verifier: "regex_match".into(),
        passed,
        detail,
    }
}

/// Check that the output parses as JSON and the dot-path resolves to the
/// expected value.
pub fn json_path(receipt: &Receipt, expected: &JsonPathExpectation) -> VerifierResult {
    let parsed: Value = match serde_json::from_str(&receipt.output) {
        Ok(v) => v,
        Err(e) => {
            return VerifierResult {
                verifier: "json_path".into(),
                passed: false,
                detail: format!("output is not JSON: {e}"),
            };
        }
    };
    let resolved = resolve_json_path(&parsed, &expected.path);
    match resolved {
        Some(value) => match &expected.value {
            Some(want) => {
                let passed = value == want;
                VerifierResult {
                    verifier: "json_path".into(),
                    passed,
                    detail: format!("{} = {} (expected {})", expected.path, value, want),
                }
            }
            None => VerifierResult {
                verifier: "json_path".into(),
                passed: true,
                detail: format!("{} = {}", expected.path, value),
            },
        },
        None => VerifierResult {
            verifier: "json_path".into(),
            passed: false,
            detail: format!("path {} not found in output JSON", expected.path),
        },
    }
}

/// Resolve a dot-separated path into a JSON value.
fn resolve_json_path<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = root;
    for key in path.split('.') {
        current = current.get(key)?;
    }
    Some(current)
}

/// Resolve a (possibly relative) path against the receipt's cwd.
fn resolve_path(receipt: &Receipt, path: &str) -> Option<String> {
    if Path::new(path).is_absolute() {
        return Some(path.to_string());
    }
    let cwd = receipt.cwd.as_ref()?;
    Some(format!("{cwd}/{path}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn receipt(output: &str, code: Option<i32>, cwd: Option<&str>) -> Receipt {
        Receipt {
            command: "true".into(),
            output: output.into(),
            exit_code: code,
            duration: Duration::from_millis(1),
            cwd: cwd.map(String::from),
        }
    }

    #[test]
    fn exit_code_passes_and_fails() {
        let ok = exit_code(
            &receipt("", Some(0), None),
            &ExitCodeExpectation { code: 0 },
        );
        assert!(ok.passed);
        let bad = exit_code(
            &receipt("", Some(1), None),
            &ExitCodeExpectation { code: 0 },
        );
        assert!(!bad.passed);
        assert!(bad.detail.contains("exit_code 1"));
        let missing = exit_code(&receipt("", None, None), &ExitCodeExpectation { code: 0 });
        assert!(!missing.passed);
        assert!(missing.detail.contains("no exit code"));
    }

    #[test]
    fn file_exists_resolves_relative_against_cwd() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("out.txt");
        std::fs::write(&file, "x").expect("write");

        let hit = file_exists(
            &receipt("", Some(0), Some(dir.path().to_str().unwrap())),
            &FileExistsExpectation {
                path: "out.txt".into(),
            },
        );
        assert!(hit.passed);

        let miss = file_exists(
            &receipt("", Some(0), Some(dir.path().to_str().unwrap())),
            &FileExistsExpectation {
                path: "nope.txt".into(),
            },
        );
        assert!(!miss.passed);

        let no_cwd = file_exists(
            &receipt("", Some(0), None),
            &FileExistsExpectation {
                path: "out.txt".into(),
            },
        );
        assert!(!no_cwd.passed);
        assert!(no_cwd.detail.contains("no cwd"));
    }

    #[test]
    fn regex_match_finds_and_reports_line() {
        let r = receipt("line one\nERROR: boom\nline three", Some(1), None);
        let hit = regex_match(
            &r,
            &RegexMatchExpectation {
                pattern: r"ERROR: \w+".into(),
            },
        );
        assert!(hit.passed);
        assert!(hit.detail.contains("line 2"));

        let miss = regex_match(
            &r,
            &RegexMatchExpectation {
                pattern: "WARN".into(),
            },
        );
        assert!(!miss.passed);
    }

    #[test]
    fn regex_match_invalid_pattern_fails_cleanly() {
        let r = receipt("x", Some(0), None);
        let bad = regex_match(
            &r,
            &RegexMatchExpectation {
                pattern: "(".into(),
            },
        );
        assert!(!bad.passed);
        assert!(bad.detail.contains("invalid regex"));
    }

    #[test]
    fn json_path_resolves_and_compares() {
        let r = receipt(r#"{"result": {"status": "ok", "count": 3}}"#, Some(0), None);
        let hit = json_path(
            &r,
            &JsonPathExpectation {
                path: "result.status".into(),
                value: Some(Value::String("ok".into())),
            },
        );
        assert!(hit.passed);

        let wrong = json_path(
            &r,
            &JsonPathExpectation {
                path: "result.status".into(),
                value: Some(Value::String("bad".into())),
            },
        );
        assert!(!wrong.passed);

        let absent = json_path(
            &r,
            &JsonPathExpectation {
                path: "result.missing".into(),
                value: None,
            },
        );
        assert!(!absent.passed);
    }

    #[test]
    fn json_path_non_json_output_fails() {
        let r = receipt("not json", Some(0), None);
        let bad = json_path(
            &r,
            &JsonPathExpectation {
                path: "a".into(),
                value: None,
            },
        );
        assert!(!bad.passed);
        assert!(bad.detail.contains("not JSON"));
    }
}
