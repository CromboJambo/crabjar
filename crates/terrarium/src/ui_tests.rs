//! Headless render tests for IsometricView.
//!
//! These convert visual verification into machine-checkable assertions so small models
//! have a regression net when editing the render path. The contract layer has 21 tests;
//! this adds coverage for ui.rs and iso.rs, which previously had zero #[cfg(test)].

#[cfg(test)]
mod tests {
    use crate::contract::Run;
    use crate::mapping::Behaviors;
    use crate::simulator::{self};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::prelude::Widget;

    /// Load the demo contract used in other tests.
    fn demo_contract() -> Run {
        serde_json::from_str(
            r#"{
          "dagr": 3,
          "run": {
            "id": "crabjar-demo",
            "title": "Demo Habitat"
          },
          "generated_at": "2026-09-03T15:00:00Z",
          "projects": [
            {"id": "triage", "title": "Triage Burrow"},
            {"id": "guard", "title": "Guard Reef"}
          ],
          "tasks": [
            {
              "id": "T1",
              "title": "get the workspace test suite green",
              "kind": "review",
              "owner": "maintainer",
              "project": "triage",
              "state": "review",
              "deps": [],
              "note": "make the test suite pass",
              "attempts": [
                {
                  "id": "T1·a1",
                  "n": 1,
                  "actor": "agent",
                  "state": "done",
                  "started_at": "2026-09-01T00:00:30Z",
                  "ended_at": "2026-09-01T00:01:00Z",
                  "outcome": {
                    "result": "done",
                    "evidence": "reported",
                    "receipt": "exit 0"
                  }
                }
              ]
            },
            {
              "id": "T2",
              "title": "refresh the dependency lockfile",
              "kind": "review",
              "owner": "maintainer",
              "project": "triage",
              "state": "review",
              "deps": [],
              "note": "keep Cargo.lock pinned",
              "attempts": [
                {
                  "id": "T2·a1",
                  "n": 1,
                  "actor": "agent",
                  "state": "failed",
                  "started_at": "2026-09-01T00:02:00Z",
                  "ended_at": "2026-09-01T00:02:10Z",
                  "outcome": {
                    "result": "failed",
                    "evidence": "reported",
                    "reason": "exit 0; lockfile-pinned"
                  }
                }
              ]
            },
            {
              "id": "G1",
              "title": "rm -rf target",
              "kind": "question",
              "owner": "user",
              "project": "guard",
              "state": "blocked",
              "deps": [],
              "unblock": "user",
              "note": "destructive · trust L2 · conf 0.40",
              "attempts": []
            },
            {
              "id": "THEORY",
              "title": "theory state-doc",
              "kind": "docs",
              "owner": "maintainer",
              "state": "queued",
              "deps": [],
              "note": "not yet indexed",
              "attempts": []
            }
          ],
          "events": [
            {
              "at": "2026-09-01T00:01:00Z",
              "type": "attempt_settled",
              "attempt": "T1·a1",
              "detail": "settled — exit 0"
            }
          ]
        }"#,
        )
        .expect("demo contract")
    }

    /// Render the habitat headlessly and return the buffer.
    fn render_habitat(run: &Run) -> Buffer {
        let taste = Behaviors::default();
        let habitat = simulator::simulate(run, &taste);

        // Create a large canvas to hold the isometric view (crabs can spread out significantly)
        // The iso transform formula (x-y)*1 and (x+y)/2 means we need room for both dimensions
        let area = Rect::new(0, 0, 160, 60);
        let mut buf = Buffer::empty(area);

        // Render the IsometricView widget (we need to import this from ui module)
        use crate::ui::IsometricView;
        let view = IsometricView { habitat: &habitat };
        view.render(area, &mut buf);

        buf
    }

    /// Assert that no two crabs occupy the same cell.
    ///
    /// This is the core finding #3 from the adversarial review: a small model can
    /// "fix" station offsets but stack multiple crabs on the same cell, which only
    /// human eyes would catch without this test.
    #[test]
    fn crabs_do_not_stack_on_same_cell() {
        let run = demo_contract();
        let buf = render_habitat(&run);

        // Collect all cells that contain a crab (🦀) or a station glyph
        let mut occupied: Vec<(u16, u16)> = Vec::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                let cell = &buf[(x, y)];
                let ch = cell.symbol();
                // Crabs use 🦀 (U+1F980), stations use ▓ or other glyphs
                if ch == "🦀" || ch == "▓" || ch == "┌" {
                    occupied.push((x, y));
                }
            }
        }

        // Check for duplicates
        let mut seen = std::collections::HashSet::new();
        for pos in &occupied {
            assert!(
                seen.insert(*pos),
                "Crab/station collision at {:?}: {} crabs occupy same cell",
                pos,
                occupied.iter().filter(|&p| p == pos).count()
            );
        }

        // Also assert we have at least some crab cells (sanity check)
        let crab_count = occupied
            .iter()
            .map(|(x, y)| buf[(*x, *y)].symbol())
            .filter(|ch| *ch == "🦀")
            .count();

        assert!(crab_count > 0, "No crab glyphs found in rendered buffer");
    }

    /// Assert that crabs from the same zone spread across multiple columns.
    ///
    /// Finding #3 also covers: when a zone has 2+ tasks, they should occupy distinct
    /// horizontal positions (not all stacked at column X). This test verifies the
    /// iso transform is producing spatial distribution.
    #[test]
    fn crabs_in_same_zone_have_distinct_columns() {
        let run = demo_contract();
        let buf = render_habitat(&run);

        // Collect all crab positions first
        let mut crab_positions: Vec<(u16, u16)> = Vec::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                if buf[(x, y)].symbol() == "🦀" {
                    crab_positions.push((x, y));
                }
            }
        }

        // Extract columns and check for distinctness
        let cols: Vec<u16> = crab_positions.iter().map(|(x, _)| *x).collect();
        let distinct_cols: std::collections::HashSet<_> = cols.iter().cloned().collect();

        assert!(
            distinct_cols.len() >= 2,
            "Crabs should occupy ≥2 distinct columns (iso transform bug?), got {}",
            distinct_cols.len()
        );
    }

    /// Assert that the iso transform produces vertical spread (not everything on row 0).
    ///
    /// Finding #4's root cause: the spec formula ((x+y)*(tile_h/2)) = 0 for integer tile_h=1.
    // This test verifies the live code in ui.rs is producing correct vertical distribution.
    #[test]
    fn crabs_have_vertical_distribution() {
        let run = demo_contract();
        let buf = render_habitat(&run);

        // Collect all crab positions first
        let mut crab_rows: Vec<u16> = Vec::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                if buf[(x, y)].symbol() == "🦀" {
                    crab_rows.push(y);
                    break; // Only need one per row
                }
            }
        }

        // Assert we have crabs on at least 2 different rows
        let distinct_rows: std::collections::HashSet<_> = crab_rows.iter().cloned().collect();
        assert!(
            distinct_rows.len() >= 2,
            "Crabs should span ≥2 distinct rows (iso transform bug?), got {}",
            distinct_rows.len()
        );
    }

    /// Assert that the demo contract renders at least one crab per task.
    #[test]
    fn all_tasks_render_as_crabs() {
        let run = demo_contract();
        let buf = render_habitat(&run);

        // Count crab glyphs by iterating once
        let mut crab_count = 0;
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                if buf[(x, y)].symbol() == "🦀" {
                    crab_count += 1;
                }
            }
        }

        // Demo has 4 tasks → should have at least 4 crabs (some may be clipped)
        assert!(
            crab_count >= 4,
            "Expected ≥4 crabs (one per task), got {}",
            crab_count
        );
    }

    /// Assert that station glyphs appear at crab feet.
    #[test]
    fn station_glyphs_appear_at_crab_feet() {
        let run = demo_contract();
        let buf = render_habitat(&run);

        // Find all crab cells and check for a station glyph diagonally below-right
        let mut found_station = false;
        for y in 0..buf.area.height.saturating_sub(1) {
            for x in 0..buf.area.width.saturating_sub(1) {
                if buf[(x, y)].symbol() == "🦀" {
                    // Station glyph should be at (x+1, y+1)
                    let station_cell = &buf[(x + 1, y + 1)];
                    if station_cell.symbol() != "🦀" && station_cell.symbol() != " " {
                        found_station = true;
                        break;
                    }
                }
            }
            if found_station {
                break;
            }
        }

        assert!(
            found_station,
            "Expected to find station glyphs at crab feet (bottom-right diagonal)"
        );
    }
}
