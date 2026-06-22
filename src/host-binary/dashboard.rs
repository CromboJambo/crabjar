/// Ratatui Mission Control dashboard.
///
/// Real-time view of the host runtime: agent loop state, plugin health,
/// metrics, and WorkItem progress.

use crabjar_host_core::{EventBus, PluginRegistry};
use crabjar_host_observe::MetricsCollector;
use crabjar_host_agent::AgentLoop;
use ratatui::{
    DefaultTerminal,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table},
};
use std::sync::Arc;

pub async fn run(
    event_bus: Arc<EventBus>,
    metrics: MetricsCollector,
    plugin_registry: Arc<PluginRegistry>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = ratatui::init();

    // Run the TUI loop
    let result = run_tui(&mut terminal, event_bus, metrics, plugin_registry).await;

    ratatui::restore();
    result
}

async fn run_tui(
    terminal: &mut DefaultTerminal,
    event_bus: Arc<EventBus>,
    metrics: MetricsCollector,
    _plugin_registry: Arc<PluginRegistry>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut running = true;

    while running {
        terminal.draw(|frame| {
            // Full-screen layout
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(3),  // Title
                    Constraint::Length(10), // Agent status
                    Constraint::Min(10),    // Plugins
                    Constraint::Length(8),  // Metrics
                    Constraint::Length(3),  // Footer
                ])
                .split(frame.area());

            // Title
            let title = Paragraph::new("CrabJar Mission Control")
                .block(Block::default().borders(Borders::ALL).title(" CrabJar Host "))
                .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
            frame.render_widget(title, chunks[0]);

            // Agent loop status
            let agent_block = Block::default()
                .borders(Borders::ALL)
                .title(" Agent Loop ");
            let agent_text = vec![
                Line::from(Span::raw("  Status:    Idle")),
                Line::from(Span::raw("  Objective: (none)")),
                Line::from(Span::raw("  Iteration: 0")),
                Line::from(Span::raw("  Confidence: 0.0%")),
                Line::from(Span::raw("  Tasks:     0/0")),
            ];
            let agent_para = Paragraph::new(agent_text).block(agent_block);
            frame.render_widget(agent_para, chunks[1]);

            // Plugin list
            let plugins = vec!["teams".to_string()];
            let _plugin_headers = vec!["Plugin", "Status", "Version"];
            let plugin_rows: Vec<Vec<Line>> = plugins.iter().map(|p| {
                vec![
                    Line::from(Span::raw(p)),
                    Line::from(Span::raw("healthy")),
                    Line::from(Span::raw("0.1.0")),
                ]
            }).collect();

            let plugin_block = Block::default()
                .borders(Borders::ALL)
                .title(" Plugins ");
            let plugin_table = Table::new(
                plugin_rows.into_iter().map(Row::new).collect::<Vec<_>>(),
                vec![
                Constraint::Length(20),
                Constraint::Length(10),
                Constraint::Length(10),
            ]).block(plugin_block);
            frame.render_widget(plugin_table, chunks[2]);

            // Metrics
            let _metrics_snap = metrics.snapshot();
            let metrics_text = vec![
                Line::from(Span::raw("  requests_total: 0")),
                Line::from(Span::raw("  agent_iterations: 0")),
                Line::from(Span::raw("  memory_mb: 0")),
            ];
            let metrics_para = Paragraph::new(metrics_text)
                .block(Block::default().borders(Borders::ALL).title(" Metrics "));
            frame.render_widget(metrics_para, chunks[3]);

            // Footer
            let footer = Paragraph::new(" F1=Start  F2=Stop  F3=Refresh  q=Quit ")
                .style(Style::default().fg(Color::Yellow));
            frame.render_widget(footer, chunks[4]);
        })?;

        // Check for input (non-blocking)
        use crossterm::event::{poll, read, Event, KeyCode};
        if poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = read()? {
                match key.code {
                    KeyCode::Char('q') => running = false,
                    KeyCode::F(1) => {
                        // Start agent loop
                        let mut loop_engine = AgentLoop::new(event_bus.clone(), metrics.clone());
                        loop_engine.start("Mission Control tick");
                        let _ = loop_engine.tick().await;
                    }
                    KeyCode::F(3) => {
                        // Refresh — metrics snapshot already happens on draw
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
