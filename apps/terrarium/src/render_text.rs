//! Text-mode renderer for terrarium (herdr/standard terminal fallback)

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use super::world::{Crab, World};

/// Text-mode renderer that works in any terminal (herdr, wezterm, etc.)
pub struct TextRenderer<'a> {
    world: &'a World,
}

impl<'a> TextRenderer<'a> {
    pub fn new(world: &'a World) -> Self {
        Self { world }
    }

    pub fn render(&self, frame: &mut Buffer, _body: Rect) -> crossterm::Result<()> {
        // Clear the buffer area
        for y in 0..frame.area().height {
            for x in 0..frame.area().width {
                if let Some(cell) = frame.cell_mut((x, y)) {
                    cell.set_char('.').set_style(
                        ratatui::style::Style::default()
                            .fg(ratatui::style::Color::Indexed(236)) // Light gray backdrop
                    );
                }
            }
        }

        // Draw each crab as a colored glyph
        for crab in &self.world.crabs {
            let x = crab.pos_col as u16;
            let y = crab.pos_row as u16;

            if x < frame.area().width && y < frame.area().height {
                if let Some(cell) = frame.cell_mut((x, y)) {
                    let fg = ratatui::style::Color::Rgb(crab.color[0], crab.color[1], crab.color[2]);
                    cell.set_char('🦀') // Crab emoji
                        .set_style(
                            ratatui::style::Style::default()
                                .fg(fg)
                                .add_modifier(ratatui::style::Modifier::BOLD)
                        );
                }
            }
        }

        // Draw HUD at bottom
        let hud_y = frame.area().height.saturating_sub(3);
        
        // Line 1: Stats
        if let Some(cell) = frame.cell_mut((0, hud_y)) {
            cell.set_char(format!("🦀 Terrarium v0.1 | Crabs: {} | Ticks: {}", 
                self.world.crab_count(), self.world.ticks))
                .set_style(ratatui::style::Style::default().fg(ratatui::style::Color::Cyan));
        }

        // Line 2: Controls
        if let Some(cell) = frame.cell_mut((0, hud_y + 1)) {
            cell.set_char("Controls: q=quit | Space=pause | +/- speed")
                .set_style(ratatui::style::Style::default().fg(ratatui::style::Color::Yellow));
        }

        // Line 3: Status
        if let Some(cell) = frame.cell_mut((0, hud_y + 2)) {
            cell.set_char("Status: Running in text mode (no RGP)")
                .set_style(ratatui::style::Style::default().fg(ratatui::style::Color::Green));
        }

        Ok(())
    }
}
