//! JavaScript source code analysis.
//!
//! Parses JS source to extract imports, exports, function declarations,
//! class definitions, global variable access, DOM API usage, network calls,
//! and obfuscation patterns. Uses pure Rust regex/string parsing — no external
//! JS parser dependency.

use analyze_common::{AnalysisProvenance, AnalysisResult, AnalyzeError, DoubtBlock};
use std::fs;

pub mod apis;
pub mod declarations;
pub mod imports;
pub mod obfuscation;
pub mod types;

pub use types::*;

// Version constant for provenance
const ANALYZE_JS_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Analyze JavaScript source code.
pub fn analyze_js(path: &str) -> Result<AnalysisResult<JsAnalysis>, AnalyzeError> {
    let source =
        fs::read_to_string(path).map_err(|_| AnalyzeError::FileNotFound(path.to_string()))?;
    let size_bytes = source.len() as u64;

    // Extract imports
    let imports = imports::extract_imports(&source);

    // Extract exports
    let exports = imports::extract_exports(&source);

    // Extract function declarations
    let functions = declarations::extract_functions(&source, &exports);

    // Extract class definitions
    let classes = declarations::extract_classes(&source);

    // Detect globals
    let globals = apis::detect_globals(&source);

    // Detect DOM APIs
    let dom_apis = apis::detect_dom_apis(&source);

    // Detect network calls
    let network_calls = apis::detect_network_calls(&source);

    // Detect obfuscation
    let obfuscation = obfuscation::detect_obfuscation(&source);

    let provenance = AnalysisProvenance::new("analyze-js", ANALYZE_JS_VERSION);
    let doubt = DoubtBlock::new("7d")
        .assume("file is valid JavaScript source")
        .blind_spot("minified code may have false positives for obfuscation")
        .blind_spot("dynamic imports evaluated at runtime not detected");

    let analysis = JsAnalysis {
        path: path.to_string(),
        size_bytes,
        imports,
        exports,
        functions,
        classes,
        globals,
        dom_apis,
        network_calls,
        obfuscation,
    };

    Ok(AnalysisResult::new(
        format!("js-{}", chrono::Utc::now().timestamp_millis()),
        provenance,
        analysis,
        doubt,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_test_file(content: &str) -> String {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.js");
        fs::write(&path, content).unwrap();
        // Leak the tempdir so it stays alive for the test duration
        let s = path.to_string_lossy().to_string();
        std::mem::forget(dir);
        s
    }

    #[test]
    fn test_extract_imports_es6() {
        let source = r#"import { foo, bar } from 'lodash';
import React from 'react';
import './style.css';"#;
        let imports = imports::extract_imports(source);
        assert!(imports.contains(&"lodash".to_string()));
        assert!(imports.contains(&"react".to_string()));
        assert!(imports.contains(&"./style.css".to_string()));
    }

    #[test]
    fn test_extract_imports_commonjs() {
        let source = r#"const fs = require('fs');
const path = require("path");"#;
        let imports = imports::extract_imports(source);
        assert!(imports.contains(&"fs".to_string()));
        assert!(imports.contains(&"path".to_string()));
    }

    #[test]
    fn test_extract_exports() {
        let source = r#"export function hello() {}
export class World {}
export const PI = 3.14;
module.exports = { add, subtract };
export { foo, bar };"#;
        let exports = imports::extract_exports(source);
        assert!(exports.contains(&"hello".to_string()));
        assert!(exports.contains(&"World".to_string()));
        assert!(exports.contains(&"PI".to_string()));
        assert!(exports.contains(&"add".to_string()));
        assert!(exports.contains(&"subtract".to_string()));
        assert!(exports.contains(&"foo".to_string()));
        assert!(exports.contains(&"bar".to_string()));
    }

    #[test]
    fn test_extract_functions() {
        let source = r#"function regular() {}
async function asyncFn() {}
const arrow = () => {};
const namedArrow = (x) => x;"#;
        let exports = imports::extract_exports(source);
        let functions = declarations::extract_functions(source, &exports);
        assert_eq!(functions.len(), 4);
        assert!(functions.iter().any(|f| f.name == "regular"));
        assert!(functions.iter().any(|f| f.name == "asyncFn" && f.is_async));
        assert!(functions.iter().any(|f| f.name == "arrow"));
        assert!(functions.iter().any(|f| f.name == "namedArrow"));
    }

    #[test]
    fn test_extract_classes() {
        let source = r#"class Animal {}
class Dog extends Animal {
    bark() {}
    sit() {}
}"#;
        let classes = declarations::extract_classes(source);
        assert_eq!(classes.len(), 2);
        let dog = classes.iter().find(|c| c.name == "Dog").unwrap();
        assert_eq!(dog.extends, Some("Animal".to_string()));
        assert!(dog.methods.iter().any(|m| m == "bark"));
        assert!(dog.methods.iter().any(|m| m == "sit"));
    }

    #[test]
    fn test_detect_globals() {
        let source = r#"window.alert('hi');
document.getElementById('x');
localStorage.setItem('k', 'v');" #;
        let globals = apis::detect_globals(source);
        assert!(globals.contains(&"window".to_string()));
        assert!(globals.contains(&"document".to_string()));
        assert!(globals.contains(&"localStorage".to_string()));
    }

    #[test]
    fn test_detect_dom_apis() {
        let source = r#"document.getElementById('app');
element.addEventListener('click', handler);
div.innerHTML = 'hello';"#;
        let apis = apis::detect_dom_apis(source);
        assert!(apis.contains(&"getElementById".to_string()));
        assert!(apis.contains(&"addEventListener".to_string()));
        assert!(apis.contains(&"innerHTML".to_string()));
    }

    #[test]
    fn test_detect_network_calls() {
        let source = r#"fetch('/api/users');
new XMLHttpRequest();
new WebSocket('ws://example.com');" #;
        let calls = apis::detect_network_calls(source);
        assert_eq!(calls.len(), 3);
        assert!(calls.iter().any(|c| c.kind == "fetch"));
        assert!(calls.iter().any(|c| c.kind == "XMLHttpRequest"));
        assert!(calls.iter().any(|c| c.kind == "WebSocket"));
    }

    #[test]
    fn test_detect_obfuscation() {
        let source = r#"eval('alert(1)');
atob('SGVsbG8=');
'he' + 'llo';"#;
        let report = obfuscation::detect_obfuscation(source);
        assert!(report.eval_usage);
        assert!(report.base64_decoding);
        assert!(report.string_concatenation);
    }

    #[test]
    fn test_full_analysis() {
        let source = r#"import { map } from 'lodash';
import React from 'react';

export function processData(data) {
    return map(data, item => item * 2);
}

class Calculator {
    add(a, b) { return a + b; }
}

fetch('/api/data');
"#;
        let path = write_test_file(source);
        let result = analyze_js(&path).unwrap();
        assert!(result.data.imports.contains(&"lodash".to_string()));
        assert!(result.data.exports.contains(&"processData".to_string()));
        assert_eq!(result.data.classes.len(), 1);
        assert!(result.data.network_calls.iter().any(|c| c.kind == "fetch"));
    }
}
