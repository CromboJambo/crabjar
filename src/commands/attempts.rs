//! Attempts CLI commands (ADR-006, record-only first cut)

use crabjar_lib::AttemptsCommand;

pub fn handle(command: AttemptsCommand) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match command {
        AttemptsCommand::Status { queue_path, theory, db_path } => {
            let queue = crabjar_terminal::TriageQueue::load(std::path::Path::new(&queue_path))
                .unwrap_or_else(|_| crabjar_terminal::TriageQueue::new(10));

            // Theory staleness: wire to the state-docs querier when the
            // theory doc is indexed; otherwise report null with a blind
            // spot (the theory doc is not yet created).
            let mut theory_staleness = serde_json::Value::Null;
            let mut theory_blind_spot =
                Some("theory state-doc not yet created or indexed".to_string());
            match rusqlite::Connection::open(&db_path) {
                Ok(conn) => {
                    if let Ok(()) = agent_context::state_docs::migrate(&conn) {
                        let querier = agent_context::state_docs::StateDocQuerier::new(
                            conn,
                            std::path::PathBuf::from("state-docs"),
                        );
                        let status = querier.staleness_status(&theory);
                        if status["last_modified"]
                            .as_str()
                            .map(|s| !s.is_empty())
                            .unwrap_or(false)
                        {
                            theory_staleness = status;
                            theory_blind_spot = None;
                        }
                    }
                }
                Err(_) => {
                    // No db yet — the theory doc is simply not indexed.
                }
            }

            let mut blind_spots = vec![
                "queue read is a point-in-time snapshot; attempts pushed after the read are not reflected".to_string(),
                "oldest age is wall-clock at invocation time".to_string(),
            ];
            if let Some(bs) = theory_blind_spot {
                blind_spots.push(bs);
            }

            Ok(json!({
                "success": true,
                "message": "attempt graph maintenance debt dashboard",
                "attempts": queue.status(),
                "theory_staleness": theory_staleness,
                "doubt": {
                    "assumptions": [
                        "the queue record is the faithful on-disk form of the unjudged attempts (JSONL, ADR-006 first cut)",
                        "judged attempts leave the queue; the durable record is the git graph + the ADR-005 stream",
                    ],
                    "blind_spots": blind_spots,
                    "last_validation": "queue record read from disk at invocation time",
                    "stale_after": "the next push or judgment to the triage queue",
                },
            }))
        }
        AttemptsCommand::Rewind { commit, workdir, trunk, dry_run } => {
            let workdir_path = std::path::Path::new(&workdir);
            let trunk = trunk.unwrap_or_else(|| {
                crabjar_terminal::GitRepo::open(workdir_path)
                    .and_then(|r| r.head())
                    .unwrap_or_default()
            });
            if trunk.is_empty() {
                return Ok(json!({
                    "success": false,
                    "error": format!(
                        "cannot resolve the trunk: {workdir} is not a git work tree and no --trunk was given"
                    ),
                    "doubt": {
                        "assumptions": [],
                        "blind_spots": ["workdir is not a git work tree; the fine tier requires one (coarse tier territory)"],
                        "last_validation": "GitRepo::open + head at invocation time",
                        "stale_after": "the next commit in the work tree",
                    },
                }));
            }

            if dry_run {
                return match crabjar_terminal::preflight_commit(workdir_path, &commit, &trunk) {
                    Ok(None) => Ok(json!({
                        "success": true,
                        "message": "dry run: workdir is not a git work tree",
                        "clean": false,
                        "verdict": "not_a_git_repo",
                        "doubt": {
                            "assumptions": ["the fine tier requires a git work tree"],
                            "blind_spots": ["coarse tier (VM destroy + restore) is not live; no rewind path available here"],
                            "last_validation": "GitRepo::open at invocation time",
                            "stale_after": "the next commit in the work tree",
                        },
                    })),
                    Ok(Some(crabjar_terminal::RewindPreflight::Clean)) => Ok(json!({
                        "success": true,
                        "message": "dry run: revert is clean",
                        "clean": true,
                        "verdict": "clean",
                        "commit": commit,
                        "trunk": trunk,
                        "doubt": {
                            "assumptions": ["the attempt commit is the commit to revert"],
                            "blind_spots": ["non-filestate (processes, network, external state) is not diffable and not checked"],
                            "last_validation": "pre-flight checks at invocation time",
                            "stale_after": "the next commit in the work tree",
                        },
                    })),
                    Ok(Some(crabjar_terminal::RewindPreflight::NotOnLine)) => Ok(json!({
                        "success": true,
                        "message": "dry run: commit is not an ancestor of the trunk",
                        "clean": false,
                        "verdict": "not_on_line",
                        "commit": commit,
                        "trunk": trunk,
                        "doubt": {
                            "assumptions": [],
                            "blind_spots": ["no defined revert on this line; a best-effort revert was refused"],
                            "last_validation": "pre-flight checks at invocation time",
                            "stale_after": "the next commit in the work tree",
                        },
                    })),
                    Ok(Some(crabjar_terminal::RewindPreflight::TrunkMoved(overlap))) => Ok(json!({
                        "success": true,
                        "message": "dry run: trunk moved in the attempt's regions",
                        "clean": false,
                        "verdict": "trunk_moved_in_regions",
                        "commit": commit,
                        "trunk": trunk,
                        "overlap": overlap,
                        "doubt": {
                            "assumptions": [],
                            "blind_spots": ["a git revert would conflict; the overlapping regions are listed"],
                            "last_validation": "pre-flight checks at invocation time",
                            "stale_after": "the next commit in the work tree",
                        },
                    })),
                    Err(e) => Ok(json!({
                        "success": false,
                        "error": e.to_string(),
                        "doubt": {
                            "assumptions": [],
                            "blind_spots": ["pre-flight could not be computed"],
                            "last_validation": "pre-flight error at invocation time",
                            "stale_after": "the next commit in the work tree",
                        },
                    })),
                };
            }

            match crabjar_terminal::rewind_commit(&commit, workdir_path, &trunk) {
                Ok(crabjar_terminal::RewindOutcome::Reverted { revert_commit, subject, regions }) => Ok(json!({
                    "success": true,
                    "message": "fine-tier rewind: attempt reverted",
                    "reverted": {
                        "commit": commit,
                        "subject": subject,
                        "revert_commit": revert_commit,
                        "regions": regions,
                    },
                    "doubt": {
                        "assumptions": ["the attempt commit is the commit to revert", "the work tree is clean enough for git revert to apply"],
                        "blind_spots": ["non-filestate (processes, network, external state) is not diffable and not rolled back"],
                        "last_validation": "pre-flight checks + git revert at invocation time",
                        "stale_after": "the next commit in the work tree",
                    },
                })),
                Ok(crabjar_terminal::RewindOutcome::Refused(refusal)) => Ok(json!({
                    "success": true,
                    "message": "rewind refused: work tree untouched",
                    "refused": refusal,
                    "doubt": {
                        "assumptions": [],
                        "blind_spots": [refusal.detail.clone()],
                        "last_validation": "pre-flight checks at invocation time",
                        "stale_after": "the next commit in the work tree",
                    },
                })),
                Err(e) => Ok(json!({
                    "success": false,
                    "error": e.to_string(),
                    "doubt": {
                        "assumptions": [],
                        "blind_spots": ["git revert failed; check the work tree state (git status) before retrying"],
                        "last_validation": "git revert error at invocation time",
                        "stale_after": "the next commit in the work tree",
                    },
                })),
            }
        }
    }
}
