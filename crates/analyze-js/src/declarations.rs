use crate::types::{ClassDecl, FunctionDecl};
use regex::Regex;

/// Extract function declarations from JavaScript source.
pub fn extract_functions(source: &str, exported: &[String]) -> Vec<FunctionDecl> {
    let mut functions = Vec::new();

    // Function declarations: function name() or async function name()
    let re_fn_decl = Regex::new(r"(?:async\s+)?function\s+(\w+)\s*\(").unwrap();
    for caps in re_fn_decl.captures_iter(source) {
        if let Some(name) = caps.get(1) {
            let is_async = caps[0].starts_with("async");
            let is_exported = exported.contains(&name.as_str().to_string());
            functions.push(FunctionDecl {
                name: name.as_str().to_string(),
                is_async,
                is_exported,
            });
        }
    }

    // Arrow functions assigned to const/let/var: const name = () => or async () =>
    let re_arrow = Regex::new(r"(?:const|let|var)\s+(\w+)\s*=\s*(?:async\s+)?\(").unwrap();
    for caps in re_arrow.captures_iter(source) {
        if let Some(name) = caps.get(1) {
            let name_str = name.as_str().to_string();
            // Avoid duplicates with function declarations
            if !functions.iter().any(|f| f.name == name_str) {
                functions.push(FunctionDecl {
                    name: name_str,
                    is_async: false,
                    is_exported: exported.contains(&name.as_str().to_string()),
                });
            }
        }
    }

    functions.sort_by(|a, b| a.name.cmp(&b.name));
    functions
}

/// Extract class definitions from JavaScript source.
pub fn extract_classes(source: &str) -> Vec<ClassDecl> {
    let mut classes = Vec::new();

    // Find class declarations - look for class name and extends clause
    let re_class_header = Regex::new(r"class\s+(\w+)(?:\s+extends\s+(\w+))?\s*\{").unwrap();

    for caps in re_class_header.captures_iter(source) {
        if let Some(name_cap) = caps.get(1) {
            let name = name_cap.as_str().to_string();
            let extends = caps.get(2).map(|m| m.as_str().to_string());

            // Extract methods by finding all method-like patterns until we hit another class or end
            let remaining = &source[caps.get(0).unwrap().start()..];
            let methods = extract_methods_for_class(remaining);

            classes.push(ClassDecl {
                name,
                extends,
                methods,
            });
        }
    }

    classes.sort_by(|a, b| a.name.cmp(&b.name));
    classes
}

fn extract_methods_for_class(source: &str) -> Vec<String> {
    let mut methods = Vec::new();

    // Look for method patterns: name() { or async name() {
    // Stop when we hit another class declaration or end of input
    let re_method = Regex::new(r"(?:async\s+)?(\w+)\s*\([^)]*\)\s*\{").unwrap();

    // Find all method candidates
    for caps in re_method.captures_iter(source) {
        if let Some(name) = caps.get(1) {
            let name_str = name.as_str().to_string();
            // Skip constructor and keywords that aren't methods
            if name_str != "constructor"
                && !["if", "else", "for", "while", "switch"].contains(&name_str.as_str())
            {
                methods.push(name_str);
            }
        }
    }

    methods.sort();
    methods.dedup();
    methods
}
