//! Standalone ratatui demo: animated crab mascot
//! 
//! The >~{,,∞,}~< scurrying crab from TRPL learning days.
//! Minimal example showing frame animation in ratatui.

use std::time::{Duration, Instant};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{Paragraph, Wrap},
};

/// The iconic TRPL crab shape
const CRAB_IDLE: &str = r#">~{,,∞,}~<"#;
const CRAB_WALK_FRAME1: &str = r#">≈{,,∞,}≈<"#;  // legs alternate
const CRAB_WALK_FRAME2: &str = r#"<~{,,∞,}~>"#;   // facing opposite (optional)

/// Animation states
#[derive(Clone)]
enum CrabState {
    Idle,
    Walking(u8),  // frame counter for leg alternation
    Thinking,     // blink eyes
    Happy,        // different shape
    Error,        // cracked shell
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    execute!(std::io::stdout(), EnterAlternateScreen)?;
    
    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend)?;
    
    let start_time = Instant::now();
    let mut state = CrabState::Idle;
    let mut tick_count = 0u8;
    
    // Animation timing: frames change every 100ms (10 FPS)
    let frame_duration = Duration::from_millis(100);
    let mut next_frame = start_time + frame_duration;
    
    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            
            // Center the crab
            let layout = Layout::default()
                .constraints([
                    Constraint::Ratio(1, 2),  // top space (for status)
                    Constraint::Min(5),       // crab section
                    Constraint::Ratio(1, 2),  // bottom space
                ])
                .split(area);
            
            let crab_area = layout[1];
            
            // Render crab based on state
            let content = match &state {
                CrabState::Idle => CRAB_IDLE.to_string(),
                CrabState::Walking(frame) => {
                    if *frame % 2 == 0 {
                        CRAB_WALK_FRAME1.to_string()
                    } else {
                        CRAB_WALK_FRAME2.to_string()
                    }
                },
                CrabState::Thinking => ">(•,∞,)<".to_string(),  // eyes closed
                CrabState::Happy => ">(^,∞,)<".to_string(),    // happy eyes
                CrabState::Error => "(x,x,x)~<".to_string(),   // broken shell
            };
            
            let paragraph = Paragraph::new(content)
                .style(Style::default().fg(Color::Cyan))
                .alignment(ratatui::layout::Alignment::Center);
            
            frame.render_widget(paragraph, crab_area);
            
            // Status line at top
            let status_line = match &state {
                CrabState::Idle => "Status: idle",
                CrabState::Walking(_) => "Status: scuttling...",
                CrabState::Thinking => "Status: thinking (blinking)",
                CrabState::Happy => "Status: happy!",
                CrabState::Error => "Status: ERROR!",
            };
            
            let status_paragraph = Paragraph::new(status_line)
                .style(Style::default().fg(Color::Yellow))
                .wrap(Wrap { trim: true });
            
            frame.render_widget(status_paragraph, layout[0]);
        })?;
        
        // Handle events (quit on ESC or Q)
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Esc | KeyCode::Char('q') => break,
                    KeyCode::Char('w') => state = CrabState::Walking(0),
                    KeyCode::Char('t') => state = CrabState::Thinking,
                    KeyCode::Char('h') => state = CrabState::Happy,
                    KeyCode::Char('e') => state = CrabState::Error,
                    KeyCode::Char('i') => state = CrabState::Idle,
                    _ => {}
                }
            }
        }
        
        // Update animation frame
        if Instant::now() >= next_frame {
            tick_count += 1;
            
            match &state {
                CrabState::Walking(_) => {
                    state = CrabState::Walking(tick_count);
                },
                _ => {}
            }
            
            next_frame = Instant::now() + frame_duration;
        }
        
        // Auto-transition: after 5 seconds of idle, start walking
        if let CrabState::Idle = &state {
            if start_time.elapsed().as_secs() >= 5 {
                state = CrabState::Walking(0);
            }
        }
    }
    
    disable_raw_mode()?;
    execute!(std::io::stdout(), LeaveAlternateScreen)?;
    
    Ok(())
}
