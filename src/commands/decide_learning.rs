//! Jev decision-making: feedback, metrics, promotion, and learnings subcommands.

use crate::commands::decide_db::{DecisionRecord, EvidenceStore};
use serde_json::json;
use std::time::SystemTime;

/// Handle feedback subcommand — update decision outcome.
pub fn handle_feedback(
    obs_hash: Option<&str>,
    outcome: Option<&str>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    if obs_hash.is_none() || outcome.is_none() {
        return Ok(json!({
            "success": false,
            "type": "decision_error",
            "error": "Missing required arguments. Usage: crabjar decide feedback --obs-hash <hash> --outcome correct|incorrect",
            "usage": ["crabjar decide feedback --obs-hash <hash> --outcome correct|incorrect"]
        }));
    }

    let db_path = get_db_path();
    let store = EvidenceStore::open(&db_path)?;

    let result = store.update_outcome(obs_hash.unwrap(), outcome.unwrap())?;

    Ok(json!({
        "success": true,
        "type": "feedback",
        "updated": result,
        "observation_hash": obs_hash,
        "outcome": outcome
    }))
}

/// Handle metrics subcommand — show efficiency metrics with optional time filter.
pub fn handle_metrics(last: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let db_path = get_db_path();
    let store = EvidenceStore::open(&db_path)?;

    // Parse time range and compute epoch cutoff
    let (since_epoch, time_window_label) = match parse_time_range(last) {
        Ok((epoch, label)) => (epoch, label),
        Err(e) => {
            return Ok(json!({
                "success": false,
                "type": "decision_error",
                "error": format!("Invalid time range '{}': {}", last, e),
                "usage": ["crabjar decide metrics [--last <time>]"]
            }));
        }
    };

    // Get method statistics (filtered by time if specified)
    let method_stats = if since_epoch > 0 {
        store.method_stats_since(since_epoch)?
    } else {
        store.method_stats()?
    };

    // Format stats for output
    let formatted_stats: Vec<serde_json::Value> = method_stats
        .iter()
        .map(|(method, count, avg_confidence)| {
            json!({
                "method": method,
                "count": count,
                "avg_confidence": avg_confidence
            })
        })
        .collect();

    Ok(json!({
        "success": true,
        "type": "metrics",
        "time_window": time_window_label,
        "method_stats": formatted_stats,
        "total_decisions": method_stats.iter().map(|(_, c, _)| *c).sum::<i64>()
    }))
}

/// Parse time range strings like "24h", "7d", "30m" into epoch cutoff.
fn parse_time_range(range: &str) -> Result<(i64, String), String> {
    if range == "all" || range.is_empty() {
        return Ok((0, "all".to_string()));
    }

    let lower = range.to_lowercase();
    if !lower.ends_with('h') && !lower.ends_with('d') && !lower.ends_with('m') {
        return Err("Must end with 'h' (hours), 'd' (days), or 'm' (minutes)".to_string());
    }

    let digits = &lower[..lower.len() - 1];
    let amount: i64 = digits.parse().map_err(|_| "Invalid number".to_string())?;
    if amount <= 0 {
        return Err("Amount must be positive".to_string());
    }

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| format!("Time error: {}", e))?
        .as_secs() as i64;

    let seconds = match lower.chars().last().unwrap() {
        'h' => amount * 3600,
        'd' => amount * 86400,
        'm' => amount * 60,
        _ => return Err("Unknown unit".to_string()),
    };

    let cutoff = now - seconds;
    Ok((cutoff, format!("last {}", range)))
}

/// Handle promote-rule subcommand — promote high-confidence decisions to deterministic rules.
pub fn handle_promote_rule(
    decision_id: Option<i64>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    if decision_id.is_none() {
        return Ok(json!({
            "success": false,
            "type": "decision_error",
            "error": "Missing required argument. Usage: crabjar decide promote-rule <decision_id>",
            "usage": ["crabjar decide promote-rule <decision_id>"]
        }));
    }

    let db_path = get_db_path();
    let store = EvidenceStore::open(&db_path)?;

    // Check if decision exists and is eligible for promotion
    let recent = store.recent_decisions(100)?;
    let target = recent.iter().find(|r| r.id == decision_id.unwrap());

    match target {
        Some(record) => {
            if record.method != "embedding" || record.confidence < 0.85 {
                return Ok(json!({
                    "success": false,
                    "type": "decision_error",
                    "error": format!(
                        "Decision {} not eligible for promotion (method={}, confidence={:.3}). \
                         Only high-confidence embedding decisions can be promoted.",
                        record.id, record.method, record.confidence
                    ),
                    "usage": ["crabjar decide promote-rule <decision_id>"]
                }));
            }

            // Generate rule ID from decision
            let rule_id = format!("promoted_{}_{}", record.id, chrono::Utc::now().timestamp());

            // In a full implementation, this would:
            // 1. Create a new rule file in the criteria directory
            // 2. Define keyword patterns based on the observation
            // 3. Update the orchestrator to use the new rule
            // For now, we log it as a promotion candidate

            Ok(json!({
                "success": true,
                "type": "rule_promotion",
                "decision_id": record.id,
                "rule_id": rule_id,
                "observation_preview": record.observation_preview,
                "decision_option": record.decision_option,
                "confidence": record.confidence,
                "status": "promoted",
                "note": "Rule promotion logged. In full implementation, this would create a deterministic rule file."
            }))
        }
        None => Ok(json!({
            "success": false,
            "type": "decision_error",
            "error": format!("Decision {} not found in recent history", decision_id.unwrap()),
            "usage": ["crabjar decide promote-rule <decision_id>"]
        })),
    }
}

/// Handle learnings subcommand — show learned invocation patterns by component.
pub fn handle_learnings(
    component: Option<&str>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let db_path = get_db_path();
    let store = EvidenceStore::open(&db_path)?;

    // If no component specified, show overall learning summary
    if component.is_none() {
        let method_stats = store.method_stats()?;
        let formatted: Vec<serde_json::Value> = method_stats
            .iter()
            .map(|(method, count, avg_conf)| {
                json!({
                    "component": method,
                    "decisions": count,
                    "avg_confidence": avg_conf
                })
            })
            .collect();

        return Ok(json!({
            "success": true,
            "type": "learnings",
            "scope": "all_components",
            "components": formatted
        }));
    }

    // Show learnings for specific component
    let records = store.learnings_by_component(component.unwrap(), 20)?;

    if records.is_empty() {
        return Ok(json!({
            "success": true,
            "type": "learnings",
            "component": component,
            "decisions": [],
            "count": 0,
            "note": "No decisions found for this component"
        }));
    }

    let formatted: Vec<serde_json::Value> = records
        .iter()
        .map(|r| {
            json!({
                "id": r.id,
                "observation_preview": r.observation_preview,
                "decision_option": r.decision_option,
                "confidence": r.confidence,
                "method": r.method,
                "computation_time_ms": r.computation_time_ms,
                "outcome": r.outcome
            })
        })
        .collect();

    Ok(json!({
        "success": true,
        "type": "learnings",
        "component": component,
        "decisions": formatted,
        "count": records.len()
    }))
}

fn get_db_path() -> String {
    let home = std::env::home_dir().unwrap_or_else(|| "/tmp".into());
    home.join(".crabjar")
        .join("jev_evidence.db")
        .to_string_lossy()
        .to_string()
}
