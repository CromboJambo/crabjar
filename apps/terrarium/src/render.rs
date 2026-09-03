//! Ratty/RGP presentation layer — the outside of the glass.
//!
//! Reads the world (grid + crabs) and emits Ratty Graphics Protocol APC
//! sequences into the ratatui buffer. Each crab is one registered RGP object
//! (a cube payload) placed at its current cell. Per frame we emit an update
//! (a gentle spin) and re-place only when the crab's anchor cell changes —
//! the cheap-motion pattern from the ratty `rubiks_cube` example.
//!
//! When the terminal does not support RGP (anything other than ratty), the
//! layer degrades to plain-text crab glyphs so the habitat stays watchable.

use ratatui::{
    Frame,
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Widget},
};
use ratatui_ratty::{ObjectFormat, RattyGraphic, RattyGraphicSettings};

use crate::obj::cube_obj;
use crate::world::World;

/// One RGP object per crab, plus the last anchor we placed it at.
struct CrabGraphic {
    graphic: RattyGraphic<'static>,
    last_anchor: Option<(u16, u16)>,
}

/// The ratty renderer.
pub struct RattyRenderer {
    crabs: Vec<CrabGraphic>,
    registered: bool,
    /// Whether the terminal supports RGP (decided once at startup).
    supported: bool,
}

impl RattyRenderer {
    /// Build a renderer with one cube graphic per crab, taking each crab's id
    /// from the world (the world owns identity — the renderer never invents it).
    ///
    /// `supported` is the result of [`detect_rgp`], which MUST run before
    /// `ratatui::init()` (crossterm would eat the terminal's reply).
    #[must_use]
    pub fn new(world: &World, supported: bool) -> Self {
        let crabs = world
            .crabs
            .iter()
            .map(|crab| {
                let settings = RattyGraphicSettings::new("crab-cube.obj")
                    .id(crab.id)
                    .format(ObjectFormat::Obj)
                    .normalize(false)
                    .animate(false)
                    .scale(0.9)
                    .depth(0.0)
                    .brightness(1.0);
                CrabGraphic {
                    graphic: RattyGraphic::new(settings),
                    last_anchor: None,
                }
            })
            .collect();
        Self {
            crabs,
            registered: false,
            supported,
        }
    }

    /// Whether the terminal supports RGP (decided once at startup).
    pub fn supported(&self) -> bool {
        self.supported
    }

    /// Register the cube payload for every crab (once, RGP terminals only).
    pub fn register_all(&mut self) -> std::io::Result<()> {
        if !self.supported || self.registered {
            return Ok(());
        }
        let obj = cube_obj().into_bytes();
        for cg in &self.crabs {
            cg.graphic.register_payload(&obj)?;
        }
        self.registered = true;
        Ok(())
    }

    /// Draw the HUD and emit the crab layer (RGP or text fallback).
    pub fn render(&mut self, frame: &mut Frame, world: &World) -> std::io::Result<()> {
        let area = frame.area();
        let header = Rect::new(area.x, area.y, area.width, 3);
        let body = Rect::new(
            area.x,
            area.y.saturating_add(3),
            area.width,
            area.height.saturating_sub(3),
        );

        self.draw_hud(frame, world, header);
        if self.supported {
            self.draw_backdrop(frame.buffer_mut(), body);
            self.emit_crabs(frame.buffer_mut(), world, body);
        } else {
            self.draw_text_crabs(frame.buffer_mut(), world, body);
        }
        Ok(())
    }

    fn draw_hud(&self, frame: &mut Frame, world: &World, area: Rect) {
        let mode = if self.supported {
            Span::styled("rgp 3d", Style::default().fg(Color::Green))
        } else {
            Span::styled(
                "text (run inside ratty for 3d)",
                Style::default().fg(Color::Yellow),
            )
        };
        let lines = vec![
            Line::from(vec![
                Span::styled(
                    "crabjar terrarium",
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw("  "),
                mode,
            ]),
            Line::from(vec![
                Span::styled("q", Style::default().fg(Color::Cyan)),
                Span::raw(": quit   "),
                Span::styled(
                    "Ctrl+Alt+Enter",
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(": 3D   "),
                Span::styled(
                    "Ctrl+Alt+P",
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(": perspective"),
            ]),
            Line::from(format!(
                "crabs: {}   grid: {}x{}   tick: {}",
                world.crab_count(),
                world.width,
                world.height,
                world.ticks
            )),
        ];
        Paragraph::new(lines)
            .block(Block::bordered().title("habitat"))
            .render(area, frame.buffer_mut());
    }

    fn draw_backdrop(&self, buf: &mut Buffer, area: Rect) {
        let style = Style::default().fg(Color::Indexed(236));
        for y in area.y..area.y.saturating_add(area.height) {
            for x in area.x..area.x.saturating_add(area.width) {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char('.').set_style(style);
                }
            }
        }
    }

    fn emit_crabs(&mut self, buf: &mut Buffer, world: &World, body: Rect) {
        let max_x = body.x.saturating_add(body.width).saturating_sub(1);
        let max_y = body.y.saturating_add(body.height).saturating_sub(1);
        for (i, crab) in world.crabs.iter().enumerate() {
            let Some(cg) = self.crabs.get_mut(i) else {
                continue;
            };
            // Map world cell -> terminal cell, clamped into the body.
            let tx = body.x.saturating_add(crab.pos_col as u16).min(max_x);
            let ty = body.y.saturating_add(crab.pos_row as u16).min(max_y);

            // Per-frame transform: a gentle spin so the cube reads as alive.
            {
                let s = cg.graphic.settings_mut();
                s.color = Some(crab.color);
                s.rotation = [
                    (crab.phase * 20.0).rem_euclid(360.0),
                    (crab.phase * 33.0).rem_euclid(360.0),
                    0.0,
                ];
                s.offset = [0.0, 0.0, 0.0];
            }

            // Emit the update (transform) every frame...
            emit_sequence(buf, tx, ty, &cg.graphic.update_sequence());
            // ...and re-place only when the anchor cell changes.
            if cg.last_anchor != Some((tx, ty)) {
                let rect = Rect::new(tx, ty, 1, 1);
                emit_sequence(buf, tx, ty, &cg.graphic.place_sequence(rect));
                cg.last_anchor = Some((tx, ty));
            }
        }
    }

    /// Text fallback: draw each crab as a glyph on its cell.
    fn draw_text_crabs(&self, buf: &mut Buffer, world: &World, body: Rect) {
        let max_x = body.x.saturating_add(body.width).saturating_sub(1);
        let max_y = body.y.saturating_add(body.height).saturating_sub(1);
        let backdrop = Style::default().fg(Color::Indexed(236));
        for y in body.y..body.y.saturating_add(body.height) {
            for x in body.x..body.x.saturating_add(body.width) {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char('.').set_style(backdrop);
                }
            }
        }
        for crab in &world.crabs {
            let tx = body.x.saturating_add(crab.pos_col as u16).min(max_x);
            let ty = body.y.saturating_add(crab.pos_row as u16).min(max_y);
            if let Some(cell) = buf.cell_mut((tx, ty)) {
                let fg = Color::Rgb(crab.color[0], crab.color[1], crab.color[2]);
                cell.set_char('C').set_style(Style::default().fg(fg).add_modifier(ratatui::style::Modifier::BOLD));
            }
        }
    }
}

/// Query the terminal for RGP support and wait briefly for the reply.
///
/// MUST be called before `ratatui::init()` — once crossterm owns stdin in
/// raw mode, the reply would be eaten by the event loop. Uses a short
/// non-blocking termios read window (150 ms) and restores the original
/// termios before returning.
///
/// Ratty answers with `ESC _ ratty;g;s;v=1;... ESC \` (see
/// ratty `protocols/graphics.md`); any other terminal stays silent.
/// Note: ratty does NOT set `TERM_PROGRAM` — it only sets
/// `TERM=xterm-256color` (ratty `src/runtime.rs`), so env detection is a no-go.
pub fn detect_rgp() -> bool {
    use std::io::{Read, Write};
    use std::mem::zeroed;
    use std::time::{Duration, Instant};

    unsafe {
        if libc::isatty(libc::STDIN_FILENO) == 0 {
            return false;
        }

        let mut orig: libc::termios = zeroed();
        if libc::tcgetattr(libc::STDIN_FILENO, &mut orig) != 0 {
            return false;
        }
        let mut nb = orig;
        // ICANON must be off: in canonical mode read() blocks for a newline
        // no matter what VMIN/VTIME say.
        nb.c_lflag &= !(libc::ICANON | libc::ECHO);
        nb.c_oflag &= !libc::OPOST;
        nb.c_cc[libc::VMIN] = 0;
        nb.c_cc[libc::VTIME] = 15; // 150 ms read window
        if libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &nb) != 0 {
            return false;
        }

        let result = (|| {
            let mut stdout = std::io::stdout();
            if stdout
                .write_all(b"\x1b_ratty;g;s\x1b\\")
                .and_then(|_| stdout.flush())
                .is_err()
            {
                return false;
            }
            let start = Instant::now();
            let mut buf = [0u8; 256];
            while start.elapsed() < Duration::from_millis(200) {
                let Ok(n) = std::io::stdin().read(&mut buf) else {
                    break;
                };
                if n == 0 {
                    continue;
                }
                if String::from_utf8_lossy(&buf[..n]).contains("ratty;g;s;v=") {
                    return true;
                }
            }
            false
        })();

        // Restore the original termios so ratatui starts with a clean stdin.
        let _ = libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &orig);
        result
    }
}

/// Prepend an APC sequence to the symbol of the cell at `(x, y)`.
///
/// Ratty's parser strips the APC out of the rendered stream; the visible
/// character is whatever the cell already held.
fn emit_sequence(buf: &mut Buffer, x: u16, y: u16, sequence: &str) {
    let Some(cell) = buf.cell_mut((x, y)) else {
        return;
    };
    let existing = cell.symbol().to_string();
    let mut symbol = String::with_capacity(sequence.len() + existing.len());
    symbol.push_str(sequence);
    symbol.push_str(&existing);
    cell.set_symbol(&symbol);
}
