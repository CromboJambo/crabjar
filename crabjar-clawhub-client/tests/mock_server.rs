//! Mock HTTP registry server for integration testing.
//! Provides ClawHub API endpoints with controlled test data.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::body::Bytes;
use axum::extract::{Path, RawQuery, State};
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::{get, Router};
use serde_json::{json, Value};
use tokio::task::JoinHandle;

/// Test skill definition.
#[derive(Debug, Clone)]
pub struct TestSkill {
    pub name: String,
    pub description: String,
    pub author: String,
    pub latest_version: String,
    pub download_count: u32,
}

/// Mock registry server that serves controlled test data.
pub struct MockRegistryServer {
    addr: String,
    handle: JoinHandle<()>,
    skills: Arc<Mutex<Vec<TestSkill>>>,
}

impl MockRegistryServer {
    /// Start a new mock registry server on a random available port.
    pub async fn start(skills: Vec<TestSkill>) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("Failed to bind");
        let addr = listener.local_addr().unwrap();

        let skills_arc = Arc::new(Mutex::new(skills));
        let server_skills = skills_arc.clone();

        let app = Router::new()
            .route("/skills", get(list_skills))
            .route("/skills/search", get(search_skills))
            .route("/skills/{name}", get(get_skill))
            .with_state(server_skills);

        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        Self {
            addr: format!("http://{}", addr),
            handle,
            skills: skills_arc,
        }
    }

    pub fn url(&self) -> &str {
        &self.addr
    }

    pub async fn stop(self) {
        self.handle.abort();
    }
}

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

async fn list_skills(
    State(skills): State<Arc<Mutex<Vec<TestSkill>>>>,
    RawQuery(query): RawQuery,
) -> Json<Value> {
    let params = parse_query_params(query);
    let skills = skills.lock().unwrap();
    let limit: u32 = params.get("limit").and_then(|v| v.parse().ok()).unwrap_or(20);

    let results: Vec<Value> = skills.iter()
        .take(limit as usize)
        .map(skill_to_json)
        .collect();

    Json(json!({ "skills": results, "total": results.len() }))
}

async fn search_skills(
    State(skills): State<Arc<Mutex<Vec<TestSkill>>>>,
    RawQuery(query): RawQuery,
) -> Json<Value> {
    let params = parse_query_params(query);
    let skills = skills.lock().unwrap();
    let query_str = params.get("q").cloned().unwrap_or_default();
    let limit: u32 = params.get("limit").and_then(|v| v.parse().ok()).unwrap_or(20);

    let results: Vec<Value> = skills.iter()
        .filter(|s| s.name.contains(&query_str) || s.description.contains(&query_str))
        .take(limit as usize)
        .map(skill_to_json)
        .collect();

    Json(json!({ "skills": results, "total": results.len() }))
}

async fn get_skill(
    State(skills): State<Arc<Mutex<Vec<TestSkill>>>>,
    Path(name): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let skills = skills.lock().unwrap();

    if let Some(skill) = skills.iter().find(|s| s.name == name) {
        Ok(Json(skill_to_json(skill)))
    } else {
        Err((StatusCode::NOT_FOUND, Json(json!({ "error": "Skill not found" })) ))
    }
}

fn skill_to_json(skill: &TestSkill) -> Value {
    json!({
        "name": skill.name,
        "description": skill.description,
        "author": skill.author,
        "latest_version": skill.latest_version,
        "download_count": skill.download_count,
        "versions": [{
            "version": skill.latest_version,
            "download_url": format!("http://localhost/downloads/{}.tar.gz", skill.name),
            "checksum": null
        }]
    })
}

/// Create a basic test skill.
pub fn make_test_skill(name: &str, description: &str) -> TestSkill {
    TestSkill {
        name: name.to_string(),
        description: description.to_string(),
        author: "test-author".to_string(),
        latest_version: "1.0.0".to_string(),
        download_count: 42,
    }
}

/// Create a basic test scenario with a few skills.
pub fn basic_skills() -> Vec<TestSkill> {
    vec![
        make_test_skill("hello-world", "A simple hello world skill"),
        make_test_skill("weather", "Get weather information"),
        make_test_skill("calculator", "Basic math operations"),
    ]
}