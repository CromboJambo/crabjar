/// Crate dependency info extracted from a Cargo.toml.
#[derive(Debug, Clone)]
pub struct CrateInfo {
    /// Crate name (from [package].name)
    pub name: String,
    /// Crate source directory
    pub manifest_path: std::path::PathBuf,
    /// Direct dependencies (resolved to package names)
    pub direct_deps: Vec<String>,
}

/// Resolve the actual package name from a dependency value.
fn resolve_dep_package_name(dep_val: &toml::Value) -> Option<String> {
    if let Some(pkg_field) = dep_val.get("package") {
        if let Some(pkg_name) = pkg_field.as_str() {
            return Some(pkg_name.to_string());
        }
    }
    if dep_val.get("workspace").and_then(toml::Value::as_bool) == Some(true) {
        return None;
    }
    if dep_val.is_str() {
        return None;
    }
    None
}

fn parse_deps_section(table: &toml::value::Table, direct_deps: &mut Vec<String>) {
    for (dep_key, dep_val) in table {
        let pkg_name = resolve_dep_package_name(dep_val);
        match pkg_name {
            Some(name) => direct_deps.push(name),
            None => {
                if dep_val.get("workspace").and_then(toml::Value::as_bool) == Some(true) {
                    continue;
                }
                direct_deps.push(dep_key.clone());
            }
        }
    }
}

/// Parse a Cargo.toml and extract the crate name and direct dependencies.
pub fn parse_crate_info(manifest_path: &std::path::Path) -> Option<CrateInfo> {
    let content = std::fs::read_to_string(manifest_path).ok()?;
    let parsed = content.parse::<toml::Value>().ok()?;

    let name = parsed
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())?
        .to_string();

    let mut direct_deps = Vec::new();

    for section in &["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(deps) = parsed.get(section) {
            if let Some(table) = deps.as_table() {
                parse_deps_section(table, &mut direct_deps);
            }
        }
    }

    // Check target-specific dependency tables
    if let Some(target_table) = parsed.get("target") {
        if let Some(targets) = target_table.as_table() {
            for (_target_name, target_val) in targets {
                for section in &["dependencies", "dev-dependencies", "build-dependencies"] {
                    if let Some(deps) = target_val.get(section) {
                        if let Some(table) = deps.as_table() {
                            parse_deps_section(table, &mut direct_deps);
                        }
                    }
                }
            }
        }
    }

    Some(CrateInfo {
        name,
        manifest_path: manifest_path.to_path_buf(),
        direct_deps,
    })
}

/// Discover all workspace member crates by scanning the workspace Cargo.toml.
pub fn discover_workspace_members(
    workspace_root: &std::path::Path,
) -> Result<Vec<CrateInfo>, String> {
    let workspace_cargo = workspace_root.join("Cargo.toml");
    let content = std::fs::read_to_string(&workspace_cargo)
        .map_err(|e| format!("Cannot read workspace Cargo.toml: {e}"))?;
    let parsed = content
        .parse::<toml::Value>()
        .map_err(|e| format!("Cannot parse workspace Cargo.toml: {e}"))?;

    let members = parsed
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
        .ok_or_else(|| "No [workspace.members] found".to_string())?;

    let mut crates = Vec::new();
    for member in members {
        let member_path = member.as_str().ok_or("Invalid member entry")?;
        let manifest_path = workspace_root.join(member_path).join("Cargo.toml");

        if manifest_path.exists() {
            if let Some(info) = parse_crate_info(&manifest_path) {
                crates.push(info);
            }
        }
    }

    Ok(crates)
}
