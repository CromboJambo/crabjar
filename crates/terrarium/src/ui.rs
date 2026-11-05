//! Phase 2 TUI — isometric terrarium with pixel-art crabs.

use std::fs;
use std::path::PathBuf;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
};

use crate::contract::Run;
use crate::mapping::Behaviors;
use crate::simulator::{self, Crab, Habitat};

pub struct TerrariumApp {
    contract_path: PathBuf,
    behaviors_path: Option<PathBuf>,
    run: Run,
    taste: Behaviors,
    habitat: Habitat,
    tidings_scroll: usize,
    running: bool,
    last_contract_mtime: std::time::SystemTime,
}

impl TerrariumApp {
    pub fn new(contract_path: PathBuf, behaviors_path: Option<PathBuf>) -> Self {
        let run = Run::load(&contract_path).expect("contract load");
        let taste = match &behaviors_path {
            Some(p) => Behaviors::load(p),
            None => Behaviors::default(),
        };
        let habitat = simulator::simulate(&run, &taste);
        let last_contract_mtime = fs::metadata(&contract_path)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        Self {
            contract_path,
            behaviors_path,
            run,
            taste,
            habitat,
            tidings_scroll: 0,
            running: true,
            last_contract_mtime,
        }
    }

    pub fn reload(&mut self) {
        let run = Run::load(&self.contract_path).expect("contract load");
        let taste = match &self.behaviors_path {
            Some(p) => Behaviors::load(p),
            None => Behaviors::default(),
        };
        self.run = run;
        self.taste = taste;
        self.habitat = simulator::simulate(&self.run, &self.taste);
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let res = self.run_terminal(&mut terminal);
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;
        res
    }

    fn run_terminal(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        while self.running {
            // Auto-reload if contract file changed
            if let Ok(meta) = fs::metadata(&self.contract_path)
                && let Ok(mtime) = meta.modified()
                && mtime != self.last_contract_mtime
            {
                self.reload();
                self.last_contract_mtime = mtime;
            }
            terminal.draw(|f| self.ui(f))?;
            if event::poll(std::time::Duration::from_millis(50))?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                match key.code {
                    KeyCode::Char('q') => self.running = false,
                    KeyCode::Char('r') => self.reload(),
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.tidings_scroll = self.tidings_scroll.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.tidings_scroll = self.tidings_scroll.saturating_add(1);
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn ui(&mut self, f: &mut Frame) {
        let title = self
            .run
            .run
            .title
            .clone()
            .unwrap_or_else(|| "terrarium".to_string());

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(f.area());

        let iso_area = chunks[0];
        f.render_widget(
            IsometricView {
                habitat: &self.habitat,
            },
            iso_area,
        );

        let tidings_lines: Vec<Line<'_>> = self
            .habitat
            .tidings
            .iter()
            .rev()
            .skip(self.tidings_scroll)
            .take(10)
            .map(|t| Line::from(Span::raw(t)))
            .collect();

        let ticker = Paragraph::new(Text::from(tidings_lines))
            .block(
                Block::default()
                    .title(" tidings (j/k to scroll, r to reload) ")
                    .borders(Borders::ALL),
            )
            .scroll((self.tidings_scroll as u16, 0));

        f.render_widget(ticker, chunks[1]);

        let title_bar = Paragraph::new(format!(
            " 🦀 {} · {} crabs · {} zones · generated {} ",
            title,
            self.habitat.crabs.len(),
            self.habitat.zones.len(),
            self.run.generated_at.as_deref().unwrap_or("?")
        ))
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL).title(" terrarium "));

        let title_area = Rect::new(0, 0, f.area().width, 1);
        f.render_widget(title_bar, title_area);
    }
}

pub struct IsometricView<'a> {
    pub habitat: &'a Habitat,
}

impl ratatui::widgets::Widget for IsometricView<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let mut items: Vec<RenderItem> = Vec::new();

        // Collect zones with positions from crab data
        for (zone_id, zone_label) in &self.habitat.zones {
            let mut origin_x = 0;
            let mut origin_y = 0;
            if let Some(first_crab) = self.habitat.crabs.iter().find(|c| c.zone == *zone_id) {
                origin_x = first_crab.zone_xy.0;
                origin_y = first_crab.zone_xy.1;
            }
            items.push(RenderItem::Zone {
                label: zone_label.clone(),
                origin_x,
                origin_y,
                depth: depth_key(origin_x, origin_y),
            });
        }

        // Add crabs
        for crab in &self.habitat.crabs {
            items.push(RenderItem::Crab {
                crab: crab.clone(),
                depth: depth_key(crab.zone_xy.0, crab.zone_xy.1),
            });
        }

        // Sort by depth (painter's algorithm)
        items.sort_by_key(|item| item.depth());

        // Track occupied cells to prevent station glyph overwrites (Finding #3).
        let mut occupied = std::collections::HashSet::new();

        for item in &items {
            match item {
                RenderItem::Zone {
                    label,
                    origin_x,
                    origin_y,
                    ..
                } => {
                    let (col, row) = iso_transform(*origin_x, *origin_y, 2, 1);
                    if col < area.right() && row < area.bottom() {
                        buf[Rect::new(col, row, 1, 1)].set_char('┌');
                        buf[Rect::new(col, row, 1, 1)].set_style(Style::default().fg(Color::Cyan));

                        if row > 0 {
                            let label_col = col.saturating_sub(label.len() as u16 / 2);
                            if label_col < area.right()
                                && label_col + label.len() as u16 <= area.right()
                            {
                                buf.set_string(
                                    label_col,
                                    row - 1,
                                    label.as_str(),
                                    Style::default().fg(Color::Cyan),
                                );
                            }
                        }
                    }
                }
                RenderItem::Crab { crab, .. } => {
                    let (col, row) = iso_transform(crab.zone_xy.0, crab.zone_xy.1, 2, 1);

                    if col < area.right() && row < area.bottom() {
                        // Determine visual style: exhaust tint overrides evidence tint
                        let has_exhaust = crab.tokens_in > 0 || crab.tokens_out > 0;
                        let style = if has_exhaust {
                            let total_tokens = crab.tokens_in + crab.tokens_out;
                            if total_tokens > 3000 || crab.cost_cents > 10.0 {
                                // Burning: red glow with underline
                                Style::default()
                                    .fg(Color::Rgb(255, 80, 80))
                                    .add_modifier(Modifier::UNDERLINED)
                            } else if crab.cost_cents > 1.0 {
                                // Moderate cost: orange
                                Style::default().fg(Color::Rgb(255, 165, 0))
                            } else {
                                // Cheap: green
                                Style::default().fg(Color::Green)
                            }
                        } else {
                            style_for_tint(&crab.tint)
                        };

                        // Claim the crab's footprint cells
                        occupied.insert((col, row));
                        buf[Rect::new(col, row, 1, 1)].set_char('🦀');
                        buf[Rect::new(col, row, 1, 1)].set_style(style);

                        if col + 1 < area.right() {
                            occupied.insert((col + 1, row));
                            buf[Rect::new(col + 1, row, 1, 1)].set_char('🦀');
                            buf[Rect::new(col + 1, row, 1, 1)].set_style(style);
                        }

                        if row + 1 < area.bottom() {
                            occupied.insert((col, row + 1));
                            buf[Rect::new(col, row + 1, 1, 1)].set_char('🦀');
                            buf[Rect::new(col, row + 1, 1, 1)].set_style(style);
                        }

                        // Burn indicator: show token count for expensive tasks
                        if has_exhaust && crab.tokens_in + crab.tokens_out > 1000 {
                            let burn_text =
                                format!("⚡{}", (crab.tokens_in + crab.tokens_out) / 100);
                            if col + 2 < area.right() && row + 1 < area.bottom() {
                                buf.set_string(
                                    col + 2,
                                    row + 1,
                                    burn_text.as_str(),
                                    Style::default().fg(Color::Rgb(255, 100, 100)),
                                );
                            }
                        }

                        // Only place station glyph if the cell is free (Finding #3 fix).
                        if col + 1 < area.right()
                            && row + 1 < area.bottom()
                            && !occupied.contains(&(col + 1, row + 1))
                        {
                            occupied.insert((col + 1, row + 1));
                            let glyph = crab.station_glyph.chars().next().unwrap_or('▓');
                            buf[Rect::new(col + 1, row + 1, 1, 1)].set_char(glyph);
                        }

                        if crab.shell_age > 0 && row > 0 {
                            let shell_text = format!("⌀{}", crab.shell_age);
                            let shell_col = col.saturating_sub(shell_text.len() as u16 / 2);
                            if shell_col < area.right() {
                                buf.set_string(
                                    shell_col,
                                    row - 1,
                                    shell_text.as_str(),
                                    Style::default().fg(Color::DarkGray),
                                );
                            }
                        }

                        if let Some(note) = &crab.note
                            && row > 1
                        {
                            let bubble = format!(" {}", truncate(note, 25));
                            let bubble_col = col.saturating_sub(bubble.len() as u16 / 2);
                            if bubble_col < area.right()
                                && bubble_col + bubble.len() as u16 <= area.right()
                            {
                                buf.set_string(
                                    bubble_col,
                                    row - 2,
                                    bubble.as_str(),
                                    Style::default().fg(Color::White),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone)]
enum RenderItem {
    Zone {
        label: String,
        origin_x: i32,
        origin_y: i32,
        depth: i32,
    },
    Crab {
        crab: Crab,
        depth: i32,
    },
}

impl RenderItem {
    fn depth(&self) -> i32 {
        match self {
            RenderItem::Zone { depth, .. } => *depth,
            RenderItem::Crab { depth, .. } => *depth,
        }
    }
}

fn iso_transform(x: i32, y: i32, origin_col: u16, origin_row: u16) -> (u16, u16) {
    // Use i32 arithmetic to handle negative intermediate values, then offset.
    let screen_col_i = (x - y) + 10; // +10 prevents negative wrap
    let screen_row_i = (x + y) / 2 + 2;
    (
        screen_col_i as u16 + origin_col,
        screen_row_i as u16 + origin_row,
    )
}

fn depth_key(x: i32, y: i32) -> i32 {
    x + y
}

fn style_for_tint(tint: &str) -> Style {
    match tint {
        "green" => Style::default().fg(Color::Green),
        "amber" => Style::default().fg(Color::Yellow),
        "orange" => Style::default().fg(Color::Rgb(255, 165, 0)),
        "gray" => Style::default().fg(Color::DarkGray),
        _ => Style::default().fg(Color::White),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", t)
    }
}
