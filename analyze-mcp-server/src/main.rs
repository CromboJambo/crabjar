use analyze_apk::ApkParser;
use analyze_common::AnalysisResult;
use analyze_har::analyze_har;
use analyze_js::analyze_js;
use analyze_native::ElfParser;
use rmcp::{handler::server::wrapper::Parameters, schemars, tool, tool_router};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BinaryRequest {
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct AnalyzeMcpServer;

#[tool_router(server_handler)]
impl AnalyzeMcpServer {
    #[tool(description = "Analyze ELF binary: header, sections, symbols, strings, dependencies")]
    fn analyze_binary(&self, params: Parameters<BinaryRequest>) -> String {
        let path = &params.0.path;
        match analyze_native::ElfParser::analyze(path) {
            Ok(result) => serde_json::to_string_pretty(&result).unwrap_or_default(),
            Err(e) => format!("Analysis error: {}", e),
        }
    }

    #[tool(description = "Analyze HTTP Archive (HAR) network capture file")]
    fn analyze_har(&self, params: Parameters<BinaryRequest>) -> String {
        let path = &params.0.path;
        match analyze_har(path) {
            Ok(result) => serde_json::to_string_pretty(&result).unwrap_or_default(),
            Err(e) => format!("Analysis error: {}", e),
        }
    }

    #[tool(description = "Analyze JavaScript source: imports, exports, functions, globals")]
    fn analyze_js(&self, params: Parameters<BinaryRequest>) -> String {
        let path = &params.0.path;
        match analyze_js(path) {
            Ok(result) => serde_json::to_string_pretty(&result).unwrap_or_default(),
            Err(e) => format!("Analysis error: {}", e),
        }
    }

    #[tool(description = "Analyze Android APK package structure and manifest")]
    fn analyze_apk(&self, params: Parameters<BinaryRequest>) -> String {
        let path = &params.0.path;
        match ApkParser::analyze(path) {
            Ok(result) => serde_json::to_string_pretty(&result).unwrap_or_default(),
            Err(e) => format!("Analysis error: {}", e),
        }
    }

    #[tool(description = "Analyze firmware binary: magic bytes, compression, sections")]
    fn analyze_firmware(&self, params: Parameters<BinaryRequest>) -> String {
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
    fn analyze_dotnet(&self, params: Parameters<BinaryRequest>) -> String {
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = AnalyzeMcpServer;

    rmcp::serve_server(server, (tokio::io::stdin(), tokio::io::stdout())).await?;

    Ok(())
}
