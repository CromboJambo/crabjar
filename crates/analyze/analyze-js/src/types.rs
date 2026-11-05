use serde::{Deserialize, Serialize};

/// Result of JavaScript analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsAnalysis {
    pub path: String,
    pub size_bytes: u64,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub functions: Vec<FunctionDecl>,
    pub classes: Vec<ClassDecl>,
    pub globals: Vec<String>,
    pub dom_apis: Vec<String>,
    pub network_calls: Vec<NetworkCall>,
    pub obfuscation: ObfuscationReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDecl {
    pub name: String,
    pub is_async: bool,
    pub is_exported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassDecl {
    pub name: String,
    pub extends: Option<String>,
    pub methods: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkCall {
    pub kind: String, // "fetch", "XMLHttpRequest", "WebSocket"
    pub url_pattern: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObfuscationReport {
    pub eval_usage: bool,
    pub base64_decoding: bool,
    pub string_concatenation: bool,
    pub suspicious_patterns: Vec<String>,
}
