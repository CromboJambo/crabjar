use std::collections::HashMap;

use crate::contract::Run;
use crate::crabs::Crab;
use crate::mapping::Behaviors;

/// Habitat — the simulated world of crabs derived from a run contract.
#[derive(Debug)]
pub struct Habitat {
    pub crabs: Vec<Crab>,
    pub zones: HashMap<String, String>,
    pub tidings: Vec<String>,
}

/// Simulate crab positions and behaviors from a run contract.
///
/// Each task attempt becomes a crab positioned in the terrarium based on its zone
/// (project) and state. The iso transform then maps these to screen coordinates.
pub fn simulate(run: &Run, _taste: &Behaviors) -> Habitat {
    let mut crabs: Vec<Crab> = Vec::new();
    let mut zones = HashMap::new();
    let mut tidings = Vec::new();

    // Map project IDs to zone positions (simple grid layout)
    for project in &run.projects {
        zones.insert(
            project.id.clone(),
            project.title.clone().unwrap_or_default(),
        );
    }

    let mut task_counter = 0;
    for task in &run.tasks {
        // Determine zone from project
        let zone = match &task.project {
            Some(p) => p.clone(),
            None => "unassigned".to_string(),
        };
        if !zones.contains_key(&zone) {
            zones.insert(zone.clone(), format!("Zone {}", zone));
        }

        // Position: spread tasks horizontally across the canvas
        let zone_xy = (task_counter * 4, 0);
        task_counter += 1;

        // Determine behavior from task state and kind
        let behavior = match task.state.as_deref() {
            Some("blocked") => "stuck",
            Some("queued") => "wait",
            Some("review") => "inspect",
            _ => "work",
        };

        // Determine tint from evidence status (simplified)
        let last_attempt = task.attempts.last();
        let tint = match last_attempt {
            Some(attempt)
                if attempt
                    .outcome
                    .as_ref()
                    .map_or(false, |o| o.evidence.as_deref() == Some("reported")) =>
            {
                "green"
            }
            Some(_) => "amber",
            None => "gray",
        };

        // Shell age: number of attempts (more attempts = older shell)
        let shell_age = task.attempts.len() as u32;

        // Station glyph based on task kind
        let station_glyph = match task.kind.as_deref() {
            Some("review") => '📋',
            Some("question") => '❓',
            Some("docs") => '📝',
            _ => '▓',
        };

        // Compute token/cost metrics from last attempt outcome (simplified)
        let tokens_in = 0u64;
        let tokens_out = 0u64;
        let cost_cents = 0.0f64;

        let crab = Crab {
            id: task.id.clone(),
            zone,
            zone_xy,
            behavior: behavior.to_string(),
            tint: tint.to_string(),
            shell_age,
            station_glyph: station_glyph.to_string(),
            note: task.note.clone(),
            tokens_in,
            tokens_out,
            cost_cents,
        };

        crabs.push(crab);

        // Generate tidings for notable events
        if let Some(attempt) = last_attempt {
            if let Some(outcome) = &attempt.outcome {
                match outcome.result.as_deref() {
                    Some("done") => tidings.push(format!(
                        "{} settled — {}",
                        task.id,
                        outcome.receipt.as_deref().unwrap_or("")
                    )),
                    Some("failed") => tidings.push(format!(
                        "{} failed — {}",
                        task.id,
                        outcome.reason.as_deref().unwrap_or("")
                    )),
                    _ => {}
                }
            }
        }
    }

    Habitat {
        crabs,
        zones,
        tidings,
    }
}
