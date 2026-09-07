//! Habitat CLI commands (spatial model via agent_context::habitat, ADR-003)

use serde_json::json;
use crabjar_lib::HabitatCommand;

pub fn handle(command: HabitatCommand) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match command {
        HabitatCommand::Snapshot { db_path } => {
            let store = agent_context::habitat::HabitatStore::open(&db_path)?;
            let snap = store.snapshot()?;
            Ok(json!({
                "success": true,
                "message": format!(
                    "habitat snapshot: {} areas, {} entities, {} open divergences",
                    snap.areas.len(),
                    snap.clutter(),
                    snap.open_divergences()
                ),
                "payload": {
                    "areas": snap.areas,
                    "entities": snap.entities,
                    "divergences": snap.divergences,
                    "clutter": snap.clutter(),
                    "open_divergences": snap.open_divergences(),
                },
                "doubt": {
                    "assumptions": [
                        "positions are user-placed or HA-area-derived; the model does not scan physical geometry",
                        "entity state strings are free-form; agent states expected to be working/blocked/idle",
                    ],
                    "blind_spots": [
                        "no physical sensor coupling yet (host-mqtt seam unconnected)",
                        "divergence records are only as good as the observations that create them",
                    ],
                    "last_validation": "snapshot read from habitat.db at invocation time",
                    "stale_after": "immediately after any entity placement or divergence record",
                },
            }))
        }
        HabitatCommand::AddArea {
            name,
            grid_w,
            grid_h,
            db_path,
        } => {
            let store = agent_context::habitat::HabitatStore::open(&db_path)?;
            let id = store.upsert_area(&name, grid_w, grid_h)?;
            Ok(json!({
                "success": true,
                "message": format!("area '{}' ready (id {})", name, id),
                "payload": { "area": { "id": id, "name": name, "grid_w": grid_w, "grid_h": grid_h } },
            }))
        }
        HabitatCommand::Place {
            id,
            area,
            kind,
            state,
            label,
            x,
            y,
            db_path,
        } => {
            let parsed_kind = agent_context::habitat::EntityKind::parse(&kind).ok_or_else(|| {
                format!(
                    "unknown entity kind '{}' (expected one of: agent, artifact, pending_guard_action, suspended_runtime, unresolved_decision)",
                    kind
                )
            })?;
            let store = agent_context::habitat::HabitatStore::open(&db_path)?;
            let area_id = store.area_id_by_name(&area)?.ok_or_else(|| {
                format!(
                    "area '{}' not found; run `crabjar habitat add-area` first",
                    area
                )
            })?;
            let entity = agent_context::habitat::HabitatEntity {
                id: id.clone(),
                area_id,
                kind: parsed_kind,
                state: state.clone(),
                label: label.clone(),
                x,
                y,
                created_at: String::new(),
                updated_at: String::new(),
            };
            store.upsert_entity(&entity)?;
            Ok(json!({
                "success": true,
                "message": format!("entity '{}' placed in '{}' at ({}, {})", id, area, x, y),
                "payload": {
                    "entity": {
                        "id": id,
                        "area": area,
                        "kind": kind,
                        "state": state,
                        "label": label,
                        "x": x,
                        "y": y,
                    }
                },
            }))
        }
        HabitatCommand::Divergence {
            area,
            description,
            db_path,
        } => {
            let store = agent_context::habitat::HabitatStore::open(&db_path)?;
            let area_id = store.area_id_by_name(&area)?.ok_or_else(|| {
                format!(
                    "area '{}' not found; run `crabjar habitat add-area` first",
                    area
                )
            })?;
            let id = store.record_divergence(area_id, &description)?;
            Ok(json!({
                "success": true,
                "message": "divergence recorded — exposed, not auto-corrected",
                "payload": { "divergence": { "id": id, "area": area, "description": description, "status": "open" } },
            }))
        }
        HabitatCommand::Resolve { id, db_path } => {
            let store = agent_context::habitat::HabitatStore::open(&db_path)?;
            store.resolve_divergence(id)?;
            Ok(json!({
                "success": true,
                "message": format!("divergence {} resolved", id),
                "payload": { "divergence": { "id": id, "status": "resolved" } },
            }))
        }
        HabitatCommand::Contract {
            queue_path,
            guard_db,
            theory,
            db_path,
            out,
        } => {
            let queue = crabjar_terminal::TriageQueue::load(std::path::Path::new(&queue_path))
                .unwrap_or_else(|_| crabjar_terminal::TriageQueue::new(10));
            let pending = crabjar_lib::habitat_contract::read_pending_actions(&guard_db);
            let theory_status =
                crabjar_lib::habitat_contract::read_theory_status(&db_path, &theory);
            let contract = crabjar_lib::habitat_contract::build_contract(
                &queue, &pending, &theory_status,
            );

            let mut message = format!(
                "habitat contract: {} tasks ({} triage, {} guard, 1 theory)",
                contract["tasks"].as_array().unwrap().len(),
                queue.len(),
                pending.len(),
            );
            if let Some(path) = &out {
                std::fs::write(path, serde_json::to_string_pretty(&contract)?)?;
                message.push_str(&format!(" → {path}"));
            }

            Ok(json!({
                "success": true,
                "message": message,
                "payload": { "contract": contract },
                "doubt": {
                    "assumptions": [
                        "a terminal receipt is the agent's own report, so settled attempts are evidence tier 'reported', not 'verified'",
                        "judged attempts leave the queue; the durable record is the git graph + the ADR-005 stream",
                        "pending actions are read from pending_queue only; executed/denied requests are not projected",
                    ],
                    "blind_spots": [
                        "queue and guard reads are point-in-time snapshots; the contract is stale the moment state moves",
                        "no VM/pane liveness yet — the coarse tier (ADR-002/003) is not live, so attempts carry no locator",
                        "the theory doc is a single task; per-section drift is not projected",
                    ],
                    "last_validation": "sources read from disk at invocation time",
                    "stale_after": "the next push, judgment, or guard action",
                },
            }))
        }
    }
}
