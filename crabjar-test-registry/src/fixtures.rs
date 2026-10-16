//! Test fixtures for mock registry server.
//!
//! Provides structured test data for ClawHub and GitHub API scenarios.

use serde::{Deserialize, Serialize};

/// A test skill definition for the mock registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSkill {
    pub name: String,
    pub description: String,
    pub author: String,
    pub latest_version: String,
    pub versions: Vec<TestSkillVersion>,
    pub download_count: u32,
}

/// A version of a test skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSkillVersion {
    pub version: String,
    pub download_url: String,
    pub checksum: Option<String>,
}

/// A test GitHub repository definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRepo {
    pub owner: String,
    pub name: String,
    pub description: String,
    pub stars: u32,
    pub manifest: Option<String>,
}

/// A complete test scenario for the mock registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestScenario {
    /// Skills available via ClawHub API
    pub skills: Vec<TestSkill>,
    /// Downloadable files (filename -> content)
    pub downloads: std::collections::HashMap<String, String>,
    /// GitHub repositories for testing
    pub github_repos: Vec<GitHubRepo>,
}

impl TestScenario {
    /// Create an empty scenario.
    pub fn new() -> Self {
        Self {
            skills: Vec::new(),
            downloads: std::collections::HashMap::new(),
            github_repos: Vec::new(),
        }
    }

    /// Add a skill to the scenario.
    pub fn add_skill(&mut self, skill: TestSkill) -> &mut Self {
        self.skills.push(skill);
        self
    }

    /// Add a downloadable file to the scenario.
    pub fn add_download(&mut self, filename: String, content: String) -> &mut Self {
        self.downloads.insert(filename, content);
        self
    }

    /// Add a GitHub repository to the scenario.
    pub fn add_github_repo(&mut self, repo: GitHubRepo) -> &mut Self {
        self.github_repos.push(repo);
        self
    }
}

/// Create a basic test skill with one version.
pub fn make_test_skill(name: &str, description: &str) -> TestSkill {
    let download_url = format!("http://localhost/downloads/{}.tar.gz", name);
    TestSkill {
        name: name.to_string(),
        description: description.to_string(),
        author: "test-author".to_string(),
        latest_version: "1.0.0".to_string(),
        versions: vec![TestSkillVersion {
            version: "1.0.0".to_string(),
            download_url,
            checksum: None,
        }],
        download_count: 42,
    }
}

/// Create a test scenario with a few skills for basic testing.
pub fn basic_scenario() -> TestScenario {
    let mut scenario = TestScenario::new();

    scenario.add_skill(make_test_skill("hello-world", "A simple hello world skill"));
    scenario.add_skill(make_test_skill("weather", "Get weather information"));
    scenario.add_skill(make_test_skill("calculator", "Basic math operations"));

    // Add downloadable content for each skill
    let skills = scenario.skills.clone();
    for skill in &skills {
        let manifest = serde_json::to_string(&serde_json::json!({
            "name": skill.name,
            "version": "1.0.0",
            "description": skill.description,
            "tools": []
        })).unwrap();

        scenario.add_download(
            format!("{}.tar.gz", skill.name),
            format!("mock tarball content for {} with manifest: {}", skill.name, manifest),
        );
    }

    scenario
}