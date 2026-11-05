use crate::types::NetworkCall;
use regex::Regex;

/// Detect usage of global browser objects.
pub fn detect_globals(source: &str) -> Vec<String> {
    use std::collections::HashSet;
    let mut globals = HashSet::new();

    // Check for access to known global objects
    let global_names = [
        "window",
        "document",
        "navigator",
        "location",
        "history",
        "screen",
        "localStorage",
        "sessionStorage",
        "indexedDB",
        "performance",
        "crypto",
        "setTimeout",
        "setInterval",
        "clearTimeout",
        "clearInterval",
        "requestAnimationFrame",
        "fetch",
        "WebSocket",
        "Worker",
        "BroadcastChannel",
    ];

    for name in global_names {
        // Use word boundary to avoid matching substrings
        let re = Regex::new(&format!(r"\b{}\b", name)).unwrap();
        if re.is_match(source) {
            globals.insert(name.to_string());
        }
    }

    let mut result: Vec<String> = globals.into_iter().collect();
    result.sort();
    result
}

/// Detect DOM API usage patterns.
pub fn detect_dom_apis(source: &str) -> Vec<String> {
    use std::collections::HashSet;
    let mut apis = HashSet::new();

    // Common DOM API patterns
    let dom_patterns = [
        ("getElementById", r"document\.getElementById"),
        (
            "getElementsByClassName",
            r"document\.getElementsByClassName",
        ),
        ("querySelector", r"(?:document|element)\.querySelector"),
        (
            "querySelectorAll",
            r"(?:document|element)\.querySelectorAll",
        ),
        ("addEventListener", r"\.addEventListener"),
        ("removeEventListener", r"\.removeEventListener"),
        ("createElement", r"document\.createElement"),
        ("innerHTML", r"\.innerHTML"),
        ("textContent", r"\.textContent"),
        ("classList", r"\.classList"),
        ("style", r"\.style\."),
    ];

    for (name, pattern) in dom_patterns {
        let re = Regex::new(pattern).unwrap();
        if re.is_match(source) {
            apis.insert(name.to_string());
        }
    }

    let mut result: Vec<String> = apis.into_iter().collect();
    result.sort();
    result
}

/// Detect network call patterns (fetch, XHR, WebSocket).
pub fn detect_network_calls(source: &str) -> Vec<NetworkCall> {
    let mut calls = Vec::new();

    // fetch() calls
    let re_fetch = Regex::new(r#"fetch\s*\(\s*['"]([^'"]+)['"]"#).unwrap();
    for caps in re_fetch.captures_iter(source) {
        if let Some(url) = caps.get(1) {
            calls.push(NetworkCall {
                kind: "fetch".to_string(),
                url_pattern: Some(url.as_str().to_string()),
            });
        }
    }

    // XMLHttpRequest
    let re_xhr = Regex::new(r"new\s+XMLHttpRequest").unwrap();
    if re_xhr.is_match(source) {
        calls.push(NetworkCall {
            kind: "XMLHttpRequest".to_string(),
            url_pattern: None,
        });
    }

    // WebSocket
    let re_ws = Regex::new(r#"new\s+WebSocket\s*\(\s*['"]([^'"]+)['"]"#).unwrap();
    for caps in re_ws.captures_iter(source) {
        if let Some(url) = caps.get(1) {
            calls.push(NetworkCall {
                kind: "WebSocket".to_string(),
                url_pattern: Some(url.as_str().to_string()),
            });
        }
    }

    calls
}
