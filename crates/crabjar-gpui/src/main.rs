//! Crabjar Multi-Agent Dashboard
//!
//! GPU-accelerated visualization of crabjar agent fleet state using GPUI.

use gpui::{
    App, Application, AppContext, Bounds, Context, IntoElement, ParentElement, Render, Styled, Window,
    WindowBounds, WindowOptions, div, px, rgb, size,
};
use tokio::sync::Mutex;

mod surface_adapter;
use surface_adapter::{CrabjarSurfaceAdapter, CrabjarAgent, AgentStatus};

/// Conductor API base URL (configurable via env)
const CONDUCTOR_URL: &str = "http://127.0.0.1:8080";

/// Application state shared across all panes
struct DashboardState {
    adapter: Mutex<CrabjarSurfaceAdapter>,
    conductor_url: String,
}

impl DashboardState {
    fn new() -> Self {
        let url = std::env::var("CONDUCTOR_URL").unwrap_or_else(|_| CONDUCTOR_URL.to_string());
        DashboardState {
            adapter: Mutex::new(CrabjarSurfaceAdapter::new("crabjar-dashboard")),
            conductor_url: url,
        }
    }

    /// Generate mock agent data for demonstration when conductor is not available
    fn generate_mock_agents(&self) -> Vec<CrabjarAgent> {
        vec![
            CrabjarAgent {
                id: "worker-1".to_string(),
                name: "hermes-agent".to_string(),
                status: AgentStatus::Working,
                capabilities: vec!["code-review".to_string(), "testing".to_string()],
                current_task: Some("Reviewing PR #42".to_string()),
                trust_level: 3,
                last_heartbeat: Some(1729150800),
            },
            CrabjarAgent {
                id: "worker-2".to_string(),
                name: "codex-agent".to_string(),
                status: AgentStatus::Idle,
                capabilities: vec!["code-generation".to_string()],
                current_task: None,
                trust_level: 2,
                last_heartbeat: Some(1729150790),
            },
            CrabjarAgent {
                id: "worker-3".to_string(),
                name: "reviewer-agent".to_string(),
                status: AgentStatus::Blocked,
                capabilities: vec!["code-review".to_string()],
                current_task: Some("Waiting for diff".to_string()),
                trust_level: 1,
                last_heartbeat: Some(1729150780),
            },
            CrabjarAgent {
                id: "worker-4".to_string(),
                name: "deployer-agent".to_string(),
                status: AgentStatus::Done,
                capabilities: vec!["deployment".to_string()],
                current_task: None,
                trust_level: 3,
                last_heartbeat: Some(1729150770),
            },
        ]
    }
}

/// Multi-agent dashboard view
struct DashboardView {
    state: DashboardState,
    agents: Vec<CrabjarAgent>,
    status: String,
}

impl DashboardView {
    fn new() -> Self {
        let state = DashboardState::new();
        let agents = state.generate_mock_agents();
        DashboardView { state, agents, status: "Mock Data".to_string() }
    }

    fn update_agents(&mut self, agents: Vec<CrabjarAgent>) {
        self.agents = agents;
    }

    fn set_status(&mut self, status: &str) {
        self.status = status.to_string();
    }
}

impl Render for DashboardView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let cols = if self.agents.len() <= 2 { 1 } else if self.agents.len() <= 4 { 2 } else { 3 };

        div()
            .flex_1()
            .child(
                div()
                    .child(format!("Crabjar Agent Dashboard — {}", self.status).into_element())
            )
            .child(
                div()
                    .flex_1()
                    .grid()
                    .children(self.agents.iter().map(|agent| {
                        let status_color = rgb(
                            (agent.status.color()[0] as u32) << 16 |
                            (agent.status.color()[1] as u32) << 8 |
                            (agent.status.color()[2] as u32)
                        );

                        div()
                            .border(px(1.0))
                            .child(
                                div()
                                    .flex()
                                    .child(agent.name.clone().into_element())
                                    .child(agent.status.label().to_string().into_element())
                            )
                    }))
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1200.0), px(800.0)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| DashboardView::new()),
            )
            .unwrap();

        // Poll conductor API for live data
        let state = DashboardState::new();
        let window_handle = window;

        cx.spawn(async move |cx| {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

                match reqwest::get(&format!("{}/workers", state.conductor_url)).await {
                    Ok(resp) if resp.status().is_success() => {
                        let json: serde_json::Value = match resp.json().await {
                            Ok(j) => j,
                            Err(_) => continue,
                        };
                        let workers = json["workers"].as_array().cloned().unwrap_or_default();

                        // Update adapter and dashboard
                        state.adapter.lock().await.update_agents(&workers);
                        let agents: Vec<CrabjarAgent> = state.adapter.lock().await.agents()
                            .values().cloned().collect();

                        window_handle.update(cx, move |dash: &mut DashboardView, _, _| {
                            dash.set_status("Live");
                            dash.update_agents(agents);
                        }).ok();
                    }
                    _ => {}
                }
            }
        }).detach();

        cx.activate(true);
    });
}
