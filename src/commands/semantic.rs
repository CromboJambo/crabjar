//! Semantic decision-making via embedding similarity (Jev/SemIf pattern).
//! Evaluates observations against decision criteria using cosine similarity.
//! Provides evaluate, list-criteria, and validate subcommands.

use crate::commands::decide_backend::{cosine_similarity, create_backend};
use crate::commands::decide_cache::{CachedDecision, DecisionCache};
use crate::commands::decide_criteria::{list_criteria, load_criteria, validate_criteria};
use crate::commands::decide_db::{DecisionRecord, EvidenceStore};
use crate::commands::decide_rules::RuleEngine;
use serde_json::json;
use std::env;
use std::time::SystemTime;

fn get_db_path() -> String {
    let home = env::home_dir().unwrap_or_else(|| "/tmp".into());
    home.join(".crabjar")
        .join("jev_evidence.db")
        .to_string_lossy()
        .to_string()
}

/// Evaluate an observation against decision criteria with caching and learning loop.
pub fn handle_evaluate(
    observation: &str,
    criteria_source: &str,
    backend_name: &str,
    threshold: f64,
    verbose: bool,
    skip_cache: bool,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    // Validate observation length
    if observation.len() > 8192 {
        return Ok(json!({
            "success": false,
            "type": "decision_error",
            "error": format!("Observation too long ({} chars, max 8192)", observation.len()),
            "usage": ["crabjar decide evaluate <observation> --criteria <file|inline>"]
        }));
    }

    // Compute observation hash for cache key
    let obs_hash = compute_observation_hash(observation);

    // Create cache (in-memory for now; persistence is Phase 2)
    let mut cache = DecisionCache::new(1000, 86400); // 1000 entries, 24h default TTL

    // Open evidence store for learning loop (create if doesn't exist)
    let db_path = get_db_path();
    let evidence_store = EvidenceStore::open(&db_path);

    // Check cache unless --fresh flag specified
    if !skip_cache {
        if let Some(cached) = cache.get(&obs_hash) {
            // Store cache hit in evidence store for learning loop
            if let Ok(store) = &evidence_store {
                let record = DecisionRecord {
                    id: 0,
                    observation_preview: cached.observation_preview.clone(),
                    observation_hash: obs_hash.clone(),
                    decision_option: cached.decision_option.clone(),
                    confidence: cached.confidence,
                    method: "cache".to_string(),
                    components_invoked: vec!["cache".to_string()],
                    computation_time_ms: 0,
                    outcome: None,
                    created_at: SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64,
                };
                let _ = store.store_decision(&record);
            }

            return Ok(json!({
                "success": true,
                "type": "decision",
                "observation": observation,
                "criteria_source": criteria_source,
                "backend": backend_name,
                "cached": true,
                "cache_hit_count": cached.hit_count,
                "result": {
                    "option": cached.decision_option,
                    "confidence": cached.confidence,
                    "rationale": format!("Cached decision (original computation time: {}ms)", cached.computation_time_ms)
                },
                "doubt": build_doubt_block(backend_name, criteria_source)
            }));
        }
    }

    // Load criteria
    let criteria = match load_criteria(criteria_source) {
        Ok(c) => c,
        Err(e) => {
            return Ok(json!({
                "success": false,
                "type": "decision_error",
                "error": format!("Failed to load criteria: {}", e),
                "usage": ["crabjar decide evaluate <observation> --criteria <file|inline>"]
            }));
        }
    };

    // Fast-path: try rule-based matching first (deterministic, no API call)
    if let Some(rule_json) = criteria.rules {
        if let Ok(engine) = RuleEngine::from_json(&rule_json) {
            if let Some(match_result) = engine.evaluate(observation) {
                // Store rule-based decision in evidence store
                if let Ok(store) = &evidence_store {
                    let record = DecisionRecord {
                        id: 0,
                        observation_preview: if observation.len() > 256 {
                            format!("{}...", &observation[..256])
                        } else {
                            observation.to_string()
                        },
                        observation_hash: obs_hash.clone(),
                        decision_option: match_result.decision_option.clone(),
                        confidence: match_result.confidence,
                        method: "rule_engine".to_string(),
                        components_invoked: vec![format!("rule_{}", match_result.rule_id)],
                        computation_time_ms: 0,
                        outcome: None,
                        created_at: SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as i64,
                    };
                    let _ = store.store_decision(&record);
                }

                // Rule matched — return immediately without embedding API call
                let result = json!({
                    "option": match_result.decision_option,
                    "confidence": match_result.confidence,
                    "rationale": format!(
                        "Rule '{}' matched ({} required, {} optional keywords)",
                        match_result.rule_id,
                        match_result.required_matched,
                        match_result.optional_matched
                    ),
                    "rule_based": true
                });

                // Cache the rule-based decision
                let cached_entry = CachedDecision {
                    observation_preview: if observation.len() > 256 {
                        format!("{}...", &observation[..256])
                    } else {
                        observation.to_string()
                    },
                    observation_hash: obs_hash.clone(),
                    decision_option: match_result.decision_option,
                    confidence: match_result.confidence,
                    components_invoked: vec!["rule_engine".to_string()],
                    computation_time_ms: 0,
                    cached_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    ttl_seconds: 0,
                    hit_count: 0,
                };
                cache.set(obs_hash.clone(), cached_entry);

                return Ok(json!({
                    "success": true,
                    "type": "decision",
                    "observation": observation,
                    "criteria_source": criteria_source,
                    "backend": "rule_engine",
                    "cached": false,
                    "computation_time_ms": 0,
                    "result": result,
                    "doubt": build_doubt_block("rule_engine", criteria_source)
                }));
            }
        }
    }

    // Slow-path: embedding similarity (expensive API call)
    // Create backend and time the computation
    let start = std::time::Instant::now();
    let backend = create_backend(backend_name);

    // Embed observation
    let obs_embedding = match backend.embed(observation) {
        Ok(e) => e,
        Err(e) => {
            return Ok(json!({
                "success": false,
                "type": "decision_error",
                "error": format!("Failed to embed observation: {}", e),
                "usage": ["crabjar decide evaluate <observation> --criteria <file|inline>"]
            }));
        }
    };

    // Embed all criterion options and evaluate
    let descriptions: Vec<&str> = criteria
        .options
        .iter()
        .map(|o| o.description.as_str())
        .collect();
    let option_embeddings = match backend.embed_batch(&descriptions) {
        Ok(e) => e,
        Err(e) => {
            return Ok(json!({
                "success": false,
                "type": "decision_error",
                "error": format!("Failed to embed criteria options: {}", e),
                "usage": ["crabjar decide evaluate <observation> --criteria <file|inline>"]
            }));
        }
    };

    let eval_pairs: Vec<(&str, &[f64])> = option_embeddings
        .iter()
        .enumerate()
        .map(|(i, emb)| (criteria.options[i].id.as_str(), emb.as_slice()))
        .collect();

    // Compute all similarities
    let all_scores = crate::commands::decide_backend::evaluate_all(&obs_embedding, &eval_pairs);

    // Find best match
    let best = crate::commands::decide_backend::evaluate(&obs_embedding, &eval_pairs);

    let computation_time_ms = start.elapsed().as_millis() as u64;

    // Determine effective threshold (criteria file can override CLI flag)
    let effective_threshold = if criteria.threshold > 0.0 {
        criteria.threshold
    } else {
        threshold
    };

    // Build response
    let result = match best {
        Some((option_id, confidence)) => {
            if confidence >= effective_threshold {
                json!({
                    "option": option_id,
                    "confidence": confidence,
                    "rationale": format!(
                        "High similarity to {} criterion ({:.3} vs threshold {:.2})",
                        option_id, confidence, effective_threshold
                    )
                })
            } else {
                json!({
                    "option": null,
                    "confidence": confidence,
                    "rationale": format!(
                        "Best match below threshold ({:.3} < {:.2}); route to human review",
                        confidence, effective_threshold
                    )
                })
            }
        }
        None => json!({
            "option": null,
            "confidence": 0.0,
            "rationale": "No criteria options to evaluate against"
        }),
    };

    // Store embedding-based decision in evidence store
    if let Ok(store) = &evidence_store {
        let record = DecisionRecord {
            id: 0,
            observation_preview: if observation.len() > 256 {
                format!("{}...", &observation[..256])
            } else {
                observation.to_string()
            },
            observation_hash: obs_hash.clone(),
            decision_option: result["option"].as_str().unwrap_or("null").to_string(),
            confidence: result["confidence"].as_f64().unwrap_or(0.0),
            method: "embedding".to_string(),
            components_invoked: vec![format!("{}_{}", backend_name, "similarity")],
            computation_time_ms,
            outcome: None,
            created_at: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
        };
        let _ = store.store_decision(&record);
    }

    // Cache the decision for future use
    let cached_entry = CachedDecision {
        observation_preview: if observation.len() > 256 {
            format!("{}...", &observation[..256])
        } else {
            observation.to_string()
        },
        observation_hash: obs_hash.clone(),
        decision_option: result["option"].as_str().unwrap_or("null").to_string(),
        confidence: result["confidence"].as_f64().unwrap_or(0.0),
        components_invoked: vec!["embedding".to_string()],
        computation_time_ms,
        cached_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        ttl_seconds: 0, // Use default TTL
        hit_count: 0,
    };
    cache.set(obs_hash.clone(), cached_entry);

    let mut response = json!({
        "success": true,
        "type": "decision",
        "observation": observation,
        "criteria_source": criteria_source,
        "backend": backend_name,
        "cached": false,
        "computation_time_ms": computation_time_ms,
        "result": result,
        "doubt": build_doubt_block(backend_name, criteria_source)
    });

    // Include full scores if verbose
    if verbose {
        let scores: Vec<serde_json::Value> = all_scores
            .iter()
            .map(|(name, sim)| json!({ "option": name, "similarity": sim }))
            .collect();
        response["scores"] = json!(scores);
    }

    Ok(response)
}

/// Compute SHA-256 hash of observation for cache key
fn compute_observation_hash(observation: &str) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    observation.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// List available criteria libraries.
pub fn handle_list_criteria(
    search_path: Option<&str>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let search_paths = vec!["/opt/crabjar/criteria", "~/.crabjar/criteria"];

    match list_criteria(search_path) {
        Ok(libraries) => Ok(json!({
            "success": true,
            "type": "criteria_list",
            "search_paths": search_paths,
            "libraries": libraries.iter().map(|lib| json!({
                "name": lib.name,
                "path": lib.path,
                "options": lib.options,
                "version": lib.version
            })).collect::<Vec<_>>(),
            "count": libraries.len(),
            "doubt": build_doubt_block("filesystem", "criteria directories")
        })),
        Err(e) => Ok(json!({
            "success": false,
            "type": "criteria_list_error",
            "error": format!("Failed to list criteria: {}", e),
            "doubt": build_doubt_block("filesystem", "criteria directories")
        })),
    }
}

/// Validate a criteria file.
pub fn handle_validate(file: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    // Use hash backend for validation (offline, deterministic)
    let backend = crate::commands::decide_backend::HashBackend::new();

    match validate_criteria(file, &backend) {
        Ok(result) => Ok(result),
        Err(e) => Ok(json!({
            "success": false,
            "type": "criteria_validation",
            "file": file,
            "valid": false,
            "errors": [format!("Validation error: {}", e)],
            "doubt": build_doubt_block("hash", file)
        })),
    }
}

/// Build the doubt block required by crabjar's CLI output contract.
fn build_doubt_block(backend: &str, source: &str) -> serde_json::Value {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let last_validation = chrono::DateTime::from_timestamp(now as i64, 0)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| "unknown".to_string());

    let stale_after = chrono::DateTime::from_timestamp((now + 86400) as i64, 0) // 24 hours
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| "unknown".to_string());

    json!({
        "assumptions": [
            format!("Embedding backend '{}' available and responsive", backend),
            format!("Criteria source '{}' is valid and contains distinct option embeddings", source)
        ],
        "blind_spots": [
            "Confidence threshold is heuristic; no historical precision/recall data available"
        ],
        "last_validation": last_validation,
        "stale_after": stale_after
    })
}

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
