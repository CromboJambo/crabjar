use regex::Regex;

/// Extract ES6 and CommonJS imports from JavaScript source.
pub fn extract_imports(source: &str) -> Vec<String> {
    let mut imports = Vec::new();

    // ES6 import statements: import { x } from 'module', import x from 'module'
    let re_import_from =
        Regex::new(r#"import\s+(?:\{[^}]+\}|[a-zA-Z_$][\w$]*)\s+from\s+['"]([^'"]+)['"]"#).unwrap();
    for caps in re_import_from.captures_iter(source) {
        if let Some(module) = caps.get(1) {
            imports.push(module.as_str().to_string());
        }
    }
    // Side-effect imports: import './style.css' (no named/default binding)
    let re_side_effect = Regex::new(r#"import\s+['"]([^'"]+)['"]"#).unwrap();
    for caps in re_side_effect.captures_iter(source) {
        if let Some(module) = caps.get(1) {
            imports.push(module.as_str().to_string());
        }
    }

    // CommonJS require() calls
    let re_require = Regex::new(r#"require\s*\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap();
    for caps in re_require.captures_iter(source) {
        if let Some(module) = caps.get(1) {
            imports.push(module.as_str().to_string());
        }
    }

    imports.sort();
    imports.dedup();
    imports
}

/// Extract exported names from JavaScript source.
pub fn extract_exports(source: &str) -> Vec<String> {
    use std::collections::HashSet;
    let mut exports = HashSet::new();

    // export function name() {}
    let re_export_fn = Regex::new(r"export\s+(?:async\s+)?function\s+(\w+)").unwrap();
    for caps in re_export_fn.captures_iter(source) {
        if let Some(name) = caps.get(1) {
            exports.insert(name.as_str().to_string());
        }
    }

    // export class name {}
    let re_export_class = Regex::new(r"export\s+class\s+(\w+)").unwrap();
    for caps in re_export_class.captures_iter(source) {
        if let Some(name) = caps.get(1) {
            exports.insert(name.as_str().to_string());
        }
    }

    // export const/let/var name = ...
    let re_export_const = Regex::new(r"export\s+(?:const|let|var)\s+(\w+)").unwrap();
    for caps in re_export_const.captures_iter(source) {
        if let Some(name) = caps.get(1) {
            exports.insert(name.as_str().to_string());
        }
    }

    // module.exports = { name: ... } or shorthand { add, subtract }
    let re_module_exports = Regex::new(r"module\.exports\s*=\s*\{([^}]*)\}").unwrap();
    if let Some(caps) = re_module_exports.captures(source) {
        let names = caps[1].to_string();
        for part in names.split(',') {
            let name = part.trim();
            // Handle shorthand: { add, subtract } or named: { add: fn(){} }
            if !name.is_empty() {
                let export_name = name.split(':').next().unwrap_or(name).trim().to_string();
                if !export_name.is_empty() {
                    exports.insert(export_name);
                }
            }
        }
    }

    // Named exports: export { a, b }
    let re_named_exports = Regex::new(r"export\s+\{([^}]*)\}").unwrap();
    if let Some(caps) = re_named_exports.captures(source) {
        let names = caps[1].to_string();
        for part in names.split(',') {
            let name = part.trim();
            if !name.is_empty() {
                exports.insert(name.to_string());
            }
        }
    }

    let mut result: Vec<String> = exports.into_iter().collect();
    result.sort();
    result
}
