//! Quick debug: print crab positions to verify spatial layout.

use crate::contract::Run;
use crate::mapping::Behaviors;
use crate::simulator::{self, Crab};
use std::collections::HashMap;

fn demo_contract() -> Run {
    serde_json::from_str(
        r#"{
          "dagr": 3,
          "run": {"id": "crabjar-demo", "title": "Demo Habitat"},
          "generated_at": "2026-09-03T15:00:00Z",
          "projects": [
            {"id": "triage", "title": "Triage Burrow"},
            {"id": "guard", "title": "Guard Reef"}
          ],
          "tasks": [
            {
              "id": "T1", "title": "get the workspace test suite green",
              "kind": "review", "owner": "maintainer", "project": "triage",
              "state": "review", "deps": [],
              "attempts": [{
                "id": "T1·a1", "n": 1, "actor": "agent", "state": "done",
                "outcome": {"result": "done", "evidence": "reported", "receipt": "exit 0"}
              }]
            },
            {
              "id": "T2", "title": "refresh the dependency lockfile",
              "kind": "review", "owner": "maintainer", "project": "triage",
              "state": "review", "deps": [],
              "attempts": [{
                "id": "T2·a1", "n": 1, "actor": "agent", "state": "failed",
                "outcome": {"result": "failed", "evidence": "reported", "reason": "exit 0; lockfile-pinned"}
              }]
            },
            {
              "id": "T3", "title": "verify the deterministic build gate",
              "kind": "review", "owner": "maintainer", "project": "triage",
              "state": "review", "deps": [],
              "attempts": [{
                "id": "T3·a1", "n": 1, "actor": "agent", "state": "failed",
                "outcome": {"result": "failed", "evidence": "reported", "reason": "exit 101"}
              }]
            },
            {
              "id": "G1", "title": "rm -rf target",
              "kind": "question", "owner": "user", "project": "guard",
              "state": "blocked", "deps": [], "unblock": "user",
              "attempts": []
            },
            {
              "id": "THEORY", "title": "theory state-doc",
              "kind": "docs", "owner": "maintainer",
              "state": "queued", "deps": [],
              "attempts": []
            }
          ],
          "events": [
            {"at": "2026-09-01T00:01:00Z", "type": "attempt_settled", "detail": "settled — exit 0"}
          ]
        }"#,
    )
    .expect("demo contract")
}

#[test]
fn debug_crab_positions() {
    let run = demo_contract();
    let taste = Behaviors::default();
    let hab = simulator::simulate(&run, &taste);

    println!("\n=== Crab Positions ===");
    for crab in &hab.crabs {
        // Compute iso screen position (same formula as ui.rs)
        let x = crab.zone_xy.0;
        let y = crab.zone_xy.1;
        let screen_col = ((x - y) * 1 + 10) as u16 + 2;
        let screen_row = ((x + y) / 2 + 2) as u16 + 1;
        println!(
            "{}: zone={}, zone_xy=({}, {}), screen=({}, {}) [{}]",
            crab.id, crab.zone, x, y, screen_col, screen_row, crab.behavior
        );
    }

    // Check for stacking: any two crabs at same screen position?
    let mut positions: HashMap<(u16, u16), Vec<String>> = HashMap::new();
    for crab in &hab.crabs {
        let x = crab.zone_xy.0;
        let y = crab.zone_xy.1;
        let screen_col = ((x - y) * 1 + 10) as u16 + 2;
        let screen_row = ((x + y) / 2 + 2) as u16 + 1;
        positions
            .entry((screen_col, screen_row))
            .or_default()
            .push(crab.id.clone());
    }

    println!("\n=== Position Collisions ===");
    let mut has_collision = false;
    for (pos, ids) in &positions {
        if ids.len() > 1 {
            println!("COLLISION at {:?}: {:?}", pos, ids);
            has_collision = true;
        }
    }
    if !has_collision {
        println!("No collisions — all crabs at distinct positions ✓");
    }

    // Check zone separation
    println!("\n=== Zone Origins ===");
    let mut zone_origins: HashMap<String, (i32, i32)> = HashMap::new();
    for crab in &hab.crabs {
        zone_origins.entry(crab.zone.clone()).or_insert(crab.zone_xy);
    }
    for (zone, origin) in &zone_origins {
        println!("{}: {:?}", zone, origin);
    }
}
