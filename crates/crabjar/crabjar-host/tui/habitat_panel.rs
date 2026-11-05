//! Habitat panel — renders the spatial habitat as low-fidelity cartography.
//!
//! ADR-003 pins the rendering constraint: information-dense, low-resolution
//! diagram of computational life. Each area is a grid; entities occupy cells
//! with a single glyph. Clutter is state, not decoration — the panel never
//! invents, removes, or rearranges entities; it only draws what the store
//! holds.
//!
//! The render is pure: `render_lines(&HabitatSnapshot) -> Vec<Line>` builds
//! the text cartography, and `HabitatPanel` owns the snapshot + a store for
//! refresh. Keeping the line builder pure makes it unit-testable without a
//! terminal.

use agent_context::habitat::{
    EntityKind, HabitatDivergence, HabitatEntity, HabitatSnapshot, HabitatStore,
};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

/// Glyph per entity kind — one character per cell, deliberately primitive.
fn kind_glyph(kind: &EntityKind) -> char {
    match kind {
        EntityKind::Agent => 'A',
        EntityKind::Artifact => 'f',
        EntityKind::PendingGuardAction => '!',
        EntityKind::SuspendedRuntime => 'S',
        EntityKind::UnresolvedDecision => '?',
    }
}

/// Color per entity kind — agents by state, others fixed.
fn entity_style(entity: &HabitatEntity) -> Style {
    let base = match entity.kind {
        EntityKind::Agent => match entity.state.as_str() {
            "working" => Color::Green,
            "blocked" => Color::Red,
            _ => Color::Cyan, // idle
        },
        EntityKind::Artifact => Color::DarkGray,
        EntityKind::PendingGuardAction => Color::Yellow,
        EntityKind::SuspendedRuntime => Color::Blue,
        EntityKind::UnresolvedDecision => Color::Magenta,
    };
    Style::default().fg(base).add_modifier(Modifier::BOLD)
}

/// Build the cartography lines for a snapshot.
///
/// Layout: one section per area — a header line (`area name (w x h)`), then
/// grid rows where each cell is two chars wide (glyph + space) so the grid
/// stays readable at coarse resolution. Entities sharing a cell stack as
/// `AB` (first wins the cell, extras appended). After the grids, a legend
/// line and an open-divergence list (exposed, never auto-corrected).
pub fn render_lines(snap: &HabitatSnapshot) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    if snap.areas.is_empty() {
        lines.push(Line::from(Span::styled(
            "habitat is empty — no areas yet",
            Style::default().fg(Color::DarkGray),
        )));
        return lines;
    }

    for area in &snap.areas {
        // Header
        lines.push(Line::from(vec![
            Span::styled(
                area.name.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  ({}x{})", area.grid_w, area.grid_h),
                Style::default().fg(Color::DarkGray),
            ),
        ]));

        // Grid: cell -> (stacked glyphs, sole occupant if exactly one)
        let mut grid: Vec<Vec<(String, Option<&HabitatEntity>)>> =
            vec![
                vec![(String::new(), None); area.grid_w.max(0) as usize];
                area.grid_h.max(0) as usize
            ];
        for e in snap.entities_in(area.id) {
            let (x, y) = (e.x as usize, e.y as usize);
            if x < grid[0].len() && y < grid.len() {
                let cell = &mut grid[y][x];
                cell.0.push(kind_glyph(&e.kind));
                cell.1 = match cell.1 {
                    None => Some(e),
                    Some(_) => None, // stacked — no single occupant
                };
            }
        }
        for row in &grid {
            let mut spans = Vec::new();
            for (cell, occupant) in row {
                if cell.is_empty() {
                    spans.push(Span::raw("  "));
                } else if let Some(e) = occupant {
                    spans.push(Span::styled(format!("{} ", cell), entity_style(e)));
                } else {
                    spans.push(Span::styled(
                        format!("{} ", cell),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
            }
            lines.push(Line::from(spans));
        }
        lines.push(Line::from(""));
    }

    // Legend
    lines.push(Line::from(vec![
        Span::styled("legend", Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::styled("A", Style::default().fg(Color::Green)),
        Span::raw(" agent  "),
        Span::styled("!", Style::default().fg(Color::Yellow)),
        Span::raw(" guard  "),
        Span::styled("f", Style::default().fg(Color::DarkGray)),
        Span::raw(" artifact  "),
        Span::styled("S", Style::default().fg(Color::Blue)),
        Span::raw(" suspended  "),
        Span::styled("?", Style::default().fg(Color::Magenta)),
        Span::raw(" decision"),
    ]));

    // Open divergences — exposed, never auto-corrected (ADR-003 decision 4)
    let open: Vec<&HabitatDivergence> = snap.divergences.iter().filter(|d| d.is_open()).collect();
    if !open.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("divergences ({} open):", open.len()),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
        for d in &open {
            lines.push(Line::from(vec![
                Span::styled("  ! ", Style::default().fg(Color::Red)),
                Span::styled(
                    truncate(&d.description, 60),
                    Style::default().fg(Color::Red),
                ),
            ]));
        }
    }

    lines
}

/// Truncate a string to `max` chars, appending `...` when cut.
///
/// Cuts on a char boundary so multi-byte input can't panic.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cut: String = s.chars().take(max).collect();
    format!("{cut}...")
}

/// A habitat panel widget: owns the snapshot and an optional store for
/// refresh. The snapshot is what gets rendered; the store is only used when
/// the user refreshes.
pub struct HabitatPanel {
    snapshot: HabitatSnapshot,
    store: Option<HabitatStore>,
}

impl HabitatPanel {
    /// Create a panel from a snapshot (no store — static render only).
    #[allow(dead_code)]
    pub fn new(snapshot: HabitatSnapshot) -> Self {
        Self {
            snapshot,
            store: None,
        }
    }

    /// Create a panel backed by a store (refreshable).
    pub fn with_store(store: HabitatStore) -> Result<Self, Box<dyn std::error::Error>> {
        let snapshot = store.snapshot()?;
        Ok(Self {
            snapshot,
            store: Some(store),
        })
    }

    /// Re-read the snapshot from the store, if one is attached.
    pub fn refresh(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref store) = self.store {
            self.snapshot = store.snapshot()?;
        }
        Ok(())
    }

    /// Borrow the current snapshot.
    #[allow(dead_code)]
    pub fn snapshot(&self) -> &HabitatSnapshot {
        &self.snapshot
    }

    /// Render the panel into the given frame area.
    pub fn render(&self, frame: &mut Frame<'_>, area: Rect) {
        let title = format!(
            " Habitat — {} entity/ies, {} open divergence/ies ",
            self.snapshot.clutter(),
            self.snapshot.open_divergences()
        );
        let block = Block::default().borders(Borders::ALL).title(title);
        let paragraph = Paragraph::new(render_lines(&self.snapshot)).block(block);
        frame.render_widget(paragraph, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_context::habitat::{HabitatArea, HabitatEntity};

    fn area(id: i64, name: &str, w: i64, h: i64) -> HabitatArea {
        HabitatArea {
            id,
            name: name.into(),
            grid_w: w,
            grid_h: h,
        }
    }

    fn entity(
        id: &str,
        area_id: i64,
        kind: EntityKind,
        state: &str,
        x: i64,
        y: i64,
    ) -> HabitatEntity {
        HabitatEntity {
            id: id.into(),
            area_id,
            kind,
            state: state.into(),
            label: id.into(),
            x,
            y,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn empty_snapshot_renders_placeholder() {
        let snap = HabitatSnapshot::default();
        let lines = render_lines(&snap);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].spans[0].content.as_ref().contains("empty"));
    }

    #[test]
    fn area_grid_places_entity_glyph() {
        let snap = HabitatSnapshot {
            areas: vec![area(1, "desk", 4, 2)],
            entities: vec![entity("a1", 1, EntityKind::Agent, "working", 1, 0)],
            divergences: vec![],
        };
        let lines = render_lines(&snap);
        // header + 2 grid rows + blank + legend
        assert_eq!(lines.len(), 5);
        // row 0, cell 1 should hold the agent glyph
        let row0 = &lines[1];
        assert!(row0.spans.iter().any(|s| s.content.as_ref() == "A "));
    }

    #[test]
    fn out_of_bounds_entity_is_ignored_not_panicked() {
        let snap = HabitatSnapshot {
            areas: vec![area(1, "desk", 2, 2)],
            entities: vec![entity("oob", 1, EntityKind::Agent, "idle", 99, 99)],
            divergences: vec![],
        };
        // Must not panic; grid renders with no glyph
        let lines = render_lines(&snap);
        assert_eq!(lines.len(), 5);
    }

    #[test]
    fn shared_cell_stacks_glyphs() {
        let snap = HabitatSnapshot {
            areas: vec![area(1, "desk", 2, 1)],
            entities: vec![
                entity("a1", 1, EntityKind::Agent, "idle", 0, 0),
                entity("g1", 1, EntityKind::PendingGuardAction, "pending", 0, 0),
            ],
            divergences: vec![],
        };
        let lines = render_lines(&snap);
        // cell (0,0) stacks A and !
        let row0 = &lines[1];
        assert!(row0.spans.iter().any(|s| s.content.as_ref() == "A! "));
    }

    #[test]
    fn open_divergence_is_listed() {
        let snap = HabitatSnapshot {
            areas: vec![area(1, "desk", 2, 1)],
            entities: vec![],
            divergences: vec![HabitatDivergence {
                id: 1,
                area_id: 1,
                description: "desk diverged".into(),
                status: "open".into(),
                created_at: String::new(),
                resolved_at: None,
            }],
        };
        let lines = render_lines(&snap);
        assert!(lines.iter().any(|l| {
            l.spans
                .iter()
                .any(|s| s.content.as_ref().contains("desk diverged"))
        }));
    }

    #[test]
    fn resolved_divergence_is_not_listed() {
        let snap = HabitatSnapshot {
            areas: vec![area(1, "desk", 2, 1)],
            entities: vec![],
            divergences: vec![HabitatDivergence {
                id: 1,
                area_id: 1,
                description: "desk diverged".into(),
                status: "resolved".into(),
                created_at: String::new(),
                resolved_at: Some("now".into()),
            }],
        };
        let lines = render_lines(&snap);
        assert!(!lines.iter().any(|l| {
            l.spans
                .iter()
                .any(|s| s.content.as_ref().contains("desk diverged"))
        }));
    }

    #[test]
    fn truncate_respects_char_boundaries() {
        let s: String = "é".repeat(100);
        let t = truncate(&s, 10);
        assert_eq!(t.chars().count(), 13); // 10 + "..."
        assert!(t.ends_with("..."));
    }
}
