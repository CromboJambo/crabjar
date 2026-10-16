//! Mock HTTP registry server for integration testing.
//!
//! Provides both ClawHub and GitHub-compatible API endpoints
//! for testing CrabJar's remote skill discovery pipeline.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::body::Bytes;
use axum::extract::{Path, RawQuery, State};
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::{get, Router};
use serde_json::{json, Value};
use tokio::task::JoinHandle;

use crate::fixtures::{GitHubRepo, TestScenario, TestSkill};

/// Mock registry server that serves controlled test data.
pub struct MockRegistryServer {
    addr: String,
    handle: JoinHandle<()>,
    scenario: Arc<Mutex<Option<TestScenario>>>,
}

impl MockRegistryServer {
    /// Start a new mock registry server on a random available port.
    pub async fn start(scenario: Option<TestScenario>) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("Failed to bind to port");
        let addr = listener.local_addr().unwrap();

        let scenario_arc = Arc::new(Mutex::new(scenario));
        let server_scenario = scenario_arc.clone();

        // Build router with all endpoints
        let app = build_router(server_scenario);

        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        Self {
            addr: format!("http://{}", addr),
            handle,
            scenario: scenario_arc,
        }
    }

    /// Get the base URL of the running server.
    pub fn url(&self) -> &str {
        &self.addr
    }

    /// Update the test scenario while the server is running.
    pub async fn set_scenario(&self, scenario: Option<TestScenario>) {
        let mut s = self.scenario.lock().unwrap();
        *s = scenario;
    }

    /// Stop the mock registry server.
    pub async fn stop(self) {
        self.handle.abort();
    }
}

fn build_router(scenario: Arc<Mutex<Option<TestScenario>>>) -> Router {
    // ClawHub endpoints
    let clawhub = Router::new()
        .route("/api/v1/skills", get(list_skills))
        .route("/api/v1/skills/search", get(search_skills))
        .route("/api/v1/skills/{name}", get(get_skill));

    // Download endpoints
    let downloads = Router::new()
        .route("/downloads/{filename}", get(download_file));

    // GitHub endpoints
    let github = Router::new()
        .route("/search/repositories", get(github_search_repos))
        .route("/repos/{owner}/{repo}", get(github_get_repo))
        .route("/repos/{owner}/{repo}/contents/manifest.json", get(github_get_manifest));

    // Merge all routers together (no root nesting in axum 0.8)
    clawhub
        .merge(downloads)
        .merge(github)
        .with_state(scenario)
}

// Query parameter parsing helper
fn parse_query_params(query: Option<String>) -> HashMap<String, String> {
    let mut params = HashMap::new();
    if let Some(q) = query {
        for pair in q.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                params.insert(key.to_string(), value.to_string());
            }
        }
    }
    params
}

// ClawHub API handlers

async fn list_skills(
    State(scenario): State<Arc<Mutex<Option<TestScenario>>>>,
    RawQuery(query): RawQuery,
) -> Json<Value> {
    let params = parse_query_params(query);
    let scenario = scenario.lock().unwrap();
    let limit: u32 = params.get("limit").and_then(|v| v.parse().ok()).unwrap_or(20);

    match scenario.as_ref() {
        Some(s) => {
            let skills: Vec<Value> = s.skills.iter()
                .take(limit as usize)
                .map(skill_to_json)
                .collect();
            Json(json!({ "skills": skills, "total": skills.len() }))
        }
        None => Json(json!({ "skills": [], "total": 0 })),
    }
}

async fn search_skills(
    State(scenario): State<Arc<Mutex<Option<TestScenario>>>>,
    RawQuery(query): RawQuery,
) -> Json<Value> {
    let params = parse_query_params(query);
    let scenario = scenario.lock().unwrap();
    let query_str = params.get("q").cloned().unwrap_or_default();
    let limit: u32 = params.get("limit").and_then(|v| v.parse().ok()).unwrap_or(20);

    match scenario.as_ref() {
        Some(s) => {
            let skills: Vec<Value> = s.skills.iter()
                .filter(|s| s.name.contains(&query_str) || s.description.contains(&query_str))
                .take(limit as usize)
                .map(skill_to_json)
                .collect();
            Json(json!({ "skills": skills, "total": skills.len() }))
        }
        None => Json(json!({ "skills": [], "total": 0 })),
    }
}

async fn get_skill(
    State(scenario): State<Arc<Mutex<Option<TestScenario>>>>,
    Path(name): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let scenario = scenario.lock().unwrap();

    match scenario.as_ref() {
        Some(s) => {
            if let Some(skill) = s.skills.iter().find(|s| s.name == name) {
                Ok(Json(skill_to_json(skill)))
            } else {
                Err((StatusCode::NOT_FOUND, Json(json!({ "error": "Skill not found" })) ))
            }
        }
        None => Err((StatusCode::NOT_FOUND, Json(json!({ "error": "Skill not found" })) )),
    }
}

async fn download_file(
    State(scenario): State<Arc<Mutex<Option<TestScenario>>>>,
    Path(filename): Path<String>,
) -> Result<(StatusCode, Bytes), (StatusCode, Json<Value>)> {
    let scenario = scenario.lock().unwrap();

    match scenario.as_ref() {
        Some(s) => {
            if let Some(content) = s.downloads.get(&filename) {
                Ok((StatusCode::OK, Bytes::from(content.clone())))
            } else {
                Err((StatusCode::NOT_FOUND, Json(json!({ "error": "File not found" })) ))
            }
        }
        None => Err((StatusCode::NOT_FOUND, Json(json!({ "error": "File not found" })) )),
    }
}

// GitHub API handlers

async fn github_search_repos(
    State(scenario): State<Arc<Mutex<Option<TestScenario>>>>,
    RawQuery(query): RawQuery,
) -> Json<Value> {
    let params = parse_query_params(query);
    let scenario = scenario.lock().unwrap();
    let query_str = params.get("q").cloned().unwrap_or_default();

    match scenario.as_ref() {
        Some(s) => {
            let repos: Vec<Value> = s.github_repos.iter()
                .filter(|r| r.name.contains(&query_str))
                .map(github_repo_to_json)
                .collect();
            Json(json!({ "items": repos, "total_count": repos.len() }))
        }
        None => Json(json!({ "items": [], "total_count": 0 })),
    }
}

async fn github_get_repo(
    State(scenario): State<Arc<Mutex<Option<TestScenario>>>>,
    Path((owner, repo)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let scenario = scenario.lock().unwrap();

    match scenario.as_ref() {
        Some(s) => {
            if let Some(r) = s.github_repos.iter().find(|r| r.owner == owner && r.name == repo) {
                Ok(Json(github_repo_to_json(r)))
            } else {
                Err((StatusCode::NOT_FOUND, Json(json!({ "message": "Not Found" })) ))
            }
        }
        None => Err((StatusCode::NOT_FOUND, Json(json!({ "message": "Not Found" })) )),
    }
}

async fn github_get_manifest(
    State(scenario): State<Arc<Mutex<Option<TestScenario>>>>,
    Path((owner, repo)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let scenario = scenario.lock().unwrap();

    match scenario.as_ref() {
        Some(s) => {
            if let Some(r) = s.github_repos.iter().find(|r| r.owner == owner && r.name == repo) {
                if let Some(manifest) = &r.manifest {
                    Ok(Json(json!({ "content": manifest })))
                } else {
                    Err((StatusCode::NOT_FOUND, Json(json!({ "message": "Not Found" })) ))
                }
            } else {
                Err((StatusCode::NOT_FOUND, Json(json!({ "message": "Not Found" })) ))
            }
        }
        None => Err((StatusCode::NOT_FOUND, Json(json!({ "message": "Not Found" })) )),
    }
}

// Helper functions

fn skill_to_json(skill: &TestSkill) -> Value {
    let versions: Vec<Value> = skill.versions.iter().map(|v| json!({
        "version": v.version,
        "download_url": v.download_url,
        "checksum": v.checksum
    })).collect();

    json!({
        "name": skill.name,
        "description": skill.description,
        "author": skill.author,
        "latest_version": skill.latest_version,
        "versions": versions,
        "download_count": skill.download_count
    })
}

fn github_repo_to_json(repo: &GitHubRepo) -> Value {
    json!({
        "full_name": format!("{}/{}", repo.owner, repo.name),
        "name": repo.name,
        "owner": { "login": repo.owner },
        "description": repo.description,
        "stargazers_count": repo.stars,
        "tags": ["crabjar-skill"]
    })
}