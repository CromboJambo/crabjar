use analyze_apk::ApkParser;
use analyze_har::analyze_har;
use analyze_js::analyze_js;
use analyze_native::ElfParser;
use rmcp::{handler::server::wrapper::Parameters, schemars, tool, tool_router};

#[derive(Debug, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
pub struct BinaryRequest {
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct AnalyzeMcpServer;

#[tool_router(server_handler)]
impl AnalyzeMcpServer {
    #[tool(description = "Analyze ELF binary: header, sections, symbols, strings, dependencies")]
    pub fn analyze_binary(&self, params: Parameters<BinaryRequest>) -> String {
        let path = &params.0.path;
        match ElfParser::analyze(path) {
            Ok(result) => serde_json::to_string_pretty(&result).unwrap_or_default(),
            Err(e) => format!("Analysis error: {}", e),
        }
    }

    #[tool(description = "Analyze HTTP Archive (HAR) network capture file")]
    pub fn analyze_har(&self, params: Parameters<BinaryRequest>) -> String {
        let path = &params.0.path;
        match analyze_har(path) {
            Ok(result) => serde_json::to_string_pretty(&result).unwrap_or_default(),
            Err(e) => format!("Analysis error: {}", e),
        }
    }

    #[tool(description = "Analyze JavaScript source: imports, exports, functions, globals")]
    pub fn analyze_js(&self, params: Parameters<BinaryRequest>) -> String {
        let path = &params.0.path;
        match analyze_js(path) {
            Ok(result) => serde_json::to_string_pretty(&result).unwrap_or_default(),
            Err(e) => format!("Analysis error: {}", e),
        }
    }

    #[tool(description = "Analyze Android APK package structure and manifest")]
    pub fn analyze_apk(&self, params: Parameters<BinaryRequest>) -> String {
        let path = &params.0.path;
        match ApkParser::analyze(path) {
            Ok(result) => serde_json::to_string_pretty(&result).unwrap_or_default(),
            Err(e) => format!("Analysis error: {}", e),
        }
    }

    #[tool(description = "Analyze firmware binary: magic bytes, compression, sections")]
    pub fn analyze_firmware(&self, params: Parameters<BinaryRequest>) -> String {
        let path = &params.0.path;
        match std::fs::read(path) {
            Ok(_data) => {
                // Firmware analysis not yet implemented
                format!("Firmware analysis for {} not yet implemented", path)
            }
            Err(e) => format!("Error reading {}: {}", path, e),
        }
    }

    #[tool(description = "Analyze .NET assembly: metadata, types, methods, dependencies")]
    pub fn analyze_dotnet(&self, params: Parameters<BinaryRequest>) -> String {
        let path = &params.0.path;
        match std::fs::read(path) {
            Ok(_data) => {
                // .NET analysis not yet implemented
                format!(".NET analysis for {} not yet implemented", path)
            }
            Err(e) => format!("Error reading {}: {}", path, e),
        }
    }
}