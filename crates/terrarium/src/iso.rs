//! Isometric renderer — Phase 2 (spec §6, §7.3).
//!
//! 2:1 isometric transform, pixel-art crab sprites, carry interpolation, depth sorting.
//! The spatial layer is *derived*, not stored — recomputed each frame from (run.json, behaviors.toml).

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::Widget,
    Frame,
};

use crate::simulator::Crab;

/// Isometric tile dimensions (2:1 aspect ratio for terminal cells).
pub const TILE_WIDTH: u16 = 2;
pub const TILE_HEIGHT: u16 = 1;

/// Convert logical (x, y) to screen (col, row) using isometric transform.
/// ```text
/// screen_col = (x - y) * (tile_w / 2) + origin_col
/// screen_row = (x + y) * (tile_h / 2) + origin_row
/// ```
pub fn iso_transform(x: i32, y: i32, origin_col: u16, origin_row: u16) -> (u16, u16) {
    let screen_col = ((x - y) * (TILE_WIDTH as i32 / 2)) as u16 + origin_col;
    let screen_row = ((x + y) * (TILE_HEIGHT as i32 / 2)) as u16 + origin_row;
    (screen_col, screen_row)
}

/// Depth key for painter's algorithm: sort by (x + y) ascending.
pub fn depth_key(x: i32, y: i32) -> i32 {
    x + y
}

/// Pixel-art crab sprite (2×2 tiles). Each cell is a tuple: (char, fg_color, bg_color).
#[derive(Debug, Clone)]
pub struct CrabSprite {
    pub top_left: (char, Option<Color>, Option<Color>),
    pub top_right: (char, Option<Color>, Option<Color>),
    pub bottom_left: (char, Option<Color>, Option<Color>),
    pub bottom_right: (char, Option<Color>, Option<Color>),
}

impl CrabSprite {
    /// Default crab sprite based on behavior.
    pub fn from_behavior(behavior: &str) -> Self {
        match behavior {
            "wait" => CrabSprite::default_idle(),
            "work" => CrabSprite::default_typing(),
            "inspect" => CrabSprite::default_inspecting(),
            "deliver" => CrabSprite::default_delivering(),
            "dig" => CrabSprite::default_digging(),
            "stuck" => CrabSprite::default_stuck(),
            "carry" => CrabSprite::default_carried(),
            "sew" => CrabSprite::default_sewing(),
            "rest" => CrabSprite::default_resting(),
            _ => CrabSprite::default_idle(),
        }
    }

    fn default_idle() -> Self {
        CrabSprite {
            top_left: ('🦀', Some(Color::Yellow), None),
            top_right: ('🦀', Some(Color::Yellow), None),
            bottom_left: ('🦀', Some(Color::Yellow), None),
            bottom_right: ('🦀', Some(Color::Yellow), None),
        }
    }

    fn default_typing() -> Self {
        CrabSprite {
            top_left: ('⌨', Some(Color::Blue), None),
            top_right: ('🦀', Some(Color::Yellow), None),
            bottom_left: ('🦀', Some(Color::Yellow), None),
            bottom_right: ('▓', Some(Color::DarkGray), None),
        }
    }

    fn default_inspecting() -> Self {
        CrabSprite {
            top_left: ('⌕', Some(Color::Green), None),
            top_right: ('🦀', Some(Color::Yellow), None),
            bottom_left: ('🦀', Some(Color::Yellow), None),
            bottom_right: ('▓', Some(Color::DarkGray), None),
        }
    }

    fn default_delivering() -> Self {
        CrabSprite {
            top_left: ('🎁', Some(Color::Green), None),
            top_right: ('🦀', Some(Color::Yellow), None),
            bottom_left: ('🦀', Some(Color::Yellow), None),
            bottom_right: ('▓', Some(Color::DarkGray), None),
        }
    }

    fn default_digging() -> Self {
        CrabSprite {
            top_left: ('⌗', Some(Color::Red), None),
            top_right: ('🦀', Some(Color::Yellow), None),
            bottom_left: ('🦀', Some(Color::Yellow), None),
            bottom_right: ('▓', Some(Color::DarkGray), None),
        }
    }

    fn default_stuck() -> Self {
        CrabSprite {
            top_left: ('❗', Some(Color::Red), None),
            top_right: ('🦀', Some(Color::Yellow), None),
            bottom_left: ('🦀', Some(Color::Yellow), None),
            bottom_right: ('▓', Some(Color::DarkGray), None),
        }
    }

    fn default_carried() -> Self {
        CrabSprite {
            top_left: ('📦', Some(Color::Brown), None),
            top_right: ('🦀', Some(Color::Yellow), None),
            bottom_left: ('🦀', Some(Color::Yellow), None),
            bottom_right: ('▓', Some(Color::DarkGray), None),
        }
    }

    fn default_sewing() -> Self {
        CrabSprite {
            top_left: ('✂', Some(Color::Purple), None),
            top_right: ('🦀', Some(Color::Yellow), None),
            bottom_left: ('🦀', Some(Color::Yellow), None),
            bottom_right: ('▓', Some(Color::DarkGray), None),
        }
    }

    fn default_resting() -> Self {
        CrabSprite {
            top_left: ('💤', Some(Color::Cyan), None),
            top_right: ('🦀', Some(Color::Yellow), None),
            bottom_left: ('🦀', Some(Color::Yellow), None),
            bottom_right: ('▓', Some(Color::DarkGray), None),
        }
    }

    /// Apply evidence tint to the crab's body cells.
    pub fn with_tint(mut self, tint: &str) -> Self {
        let fg = match tint {
            "green" => Some(Color::Green),
            "amber" => Some(Color::Yellow),
            "orange" => Some(Color::Rgb(255, 165, 0)),
            "gray" => Some(Color::DarkGray),
            _ => Some(Color::Yellow),
        };
        self.top_right = (self.top_right.0, fg, self.top_right.2);
        self.bottom_left = (self.bottom_left.0, fg, self.bottom_left.2);
        self
    }

    /// Apply shell age tint (darker = older).
    pub fn with_shell_age(mut self, age: u32) -> Self {
        // Darken the crab body based on shell age
        let shade = if age > 5 { Color::DarkGray } else { Color::Yellow };
        self.top_right = (self.top_right.0, Some(shade), self.top_right.2);
        self.bottom_left = (self.bottom_left.0, Some(shade), self.bottom_left.2);
        self
    }
}

/// Render a crab at its home zone origin + station offset.
pub struct CrabWidget {
    pub crab: Crab,
    pub screen_pos: (u16, u16),
    pub sprite: CrabSprite,
}

impl CrabWidget {
    pub fn new(crab: &Crab, origin_col: u16, origin_row: u16) -> Self {
        let (screen_col, screen_row) = iso_transform(
            crab.zone_xy.0,
            crab.zone_xy.1,
            origin_col,
            origin_row,
        );
        let sprite = CrabSprite::from_behavior(&crab.behavior)
            .with_tint(&crab.tint)
            .with_shell_age(crab.shell_age);
        Self {
            crab: crab.clone(),
            screen_pos: (screen_col, screen_row),
            sprite,
        }
    }
}

impl Widget for CrabWidget {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let (col, row) = self.screen_pos;

        // Top-left crab cell
        buf.get_mut(col, row).set_char(self.sprite.top_left.0);
        if let Some(fg) = self.sprite.top_left.1 {
            buf.get_mut(col, row).set_fg(fg);
        }

        // Top-right crab cell (one column right)
        if col + 1 < area.right() {
            buf.get_mut(col + 1, row).set_char(self.sprite.top_right.0);
            if let Some(fg) = self.sprite.top_right.1 {
                buf.get_mut(col + 1, row).set_fg(fg);
            }
        }

        // Bottom-left crab cell (one row down)
        if row + 1 < area.bottom() {
            buf.get_mut(col, row + 1).set_char(self.sprite.bottom_left.0);
            if let Some(fg) = self.sprite.bottom_left.1 {
                buf.get_mut(col, row + 1).set_fg(fg);
            }
        }

        // Bottom-right crab cell (diagonal)
        if col + 1 < area.right() && row + 1 < area.bottom() {
            buf.get_mut(col + 1, row + 1).set_char(self.sprite.bottom_right.0);
            if let Some(fg) = self.sprite.bottom_right.1 {
                buf.get_mut(col + 1, row + 1).set_fg(fg);
            }
        }

        // Draw station glyph at crab's feet (bottom-right corner)
        if col + 1 < area.right() && row + 1 < area.bottom() {
            buf.get_mut(col + 1, row + 1).set_char(self.crab.station_glyph.chars().next().unwrap_or('▓'));
        }
    }
}

/// Zone floor renderer — draws a labeled region for each project.
pub struct ZoneWidget {
    pub zone_id: String,
    pub label: String,
    pub origin_x: i32,
    pub origin_y: i32,
    pub width: i32,
    pub height: i32,
}

impl Widget for ZoneWidget {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let origin_col = ((self.origin_x - self.origin_y) * (TILE_WIDTH as i32 / 2)) as u16;
        let origin_row = ((self.origin_x + self.origin_y) * (TILE_HEIGHT as i32 / 2)) as u16;

        // Draw zone border (simplified: just a corner marker for now)
        if origin_col < area.right() && origin_row < area.bottom() {
            buf.get_mut(origin_col, origin_row).set_char('┌');
            buf.get_mut(origin_col, origin_row)
                .set_style(Style::default().fg(Color::Cyan));

            // Zone label above the corner
            if origin_row > 0 {
                let label_area = area.intersection(Rect::new(
                    origin_col.saturating_sub(self.label.len() as u16 / 2),
                    origin_row.saturating_sub(1),
                    self.label.len() as u16,
                    1,
                ));
                if label_area.area() > 0 {
                    buf.set_string(label_area, &self.label, Style::default().fg(Color::Cyan));
                }
            }
        }
    }
}
