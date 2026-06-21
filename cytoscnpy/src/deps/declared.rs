use serde::{Deserialize, Serialize};
use std::path::Path;
use toml::Value;

/// Origin of a declared dependency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencySource {
    /// Declared in pyproject.toml.
    Pyproject,
    /// Declared in a requirements.txt file.
    Requirements(String),
}

/// Represents a dependency declared in the project configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredDependency {
    /// The raw package name as it appears in the declaration.
    pub package_name: String,
    /// The normalized package name for comparison.
    pub normalized_name: String,
    /// Whether this is a development dependency.
    pub is_dev: bool,
    /// Whether this dependency came from an optional runtime extra.
    pub is_optional: bool,
    /// PEP 508 environment marker, if present.
    pub marker: Option<String>,
    /// The source file or location of the declaration.
    pub source: DependencySource,
}

/// Normalizes a package name according to PEP 503.
pub fn normalize_package_name(name: &str) -> String {
    name.to_lowercase().replace(['-', '.'], "_")
}

/// Extracts the clean package name from a PEP 508 specification string.
pub fn extract_package_name_from_pep508(spec: &str) -> Option<String> {
    extract_pep508_parts(spec).map(|(name, _)| name)
}

fn extract_pep508_parts(spec: &str) -> Option<(String, Option<String>)> {
    let spec = spec.trim();
    if spec.is_empty() || spec.starts_with('#') {
        return None;
    }

    // Skip VCS requirements (git+https://, hg+https://, svn+..., bzr+...)
    // and bare URL requirements (https://, http://) — these have no PyPI package name.
    let lower = spec.to_ascii_lowercase();
    if lower.starts_with("git+")
        || lower.starts_with("hg+")
        || lower.starts_with("svn+")
        || lower.starts_with("bzr+")
        || lower.starts_with("http://")
        || lower.starts_with("https://")
    {
        return None;
    }

    // Extract everything before version specifiers, extras, env markers, or URL separators.
    // Stop chars: `@` handles `pkg @ https://...`, `(` handles `pkg(>=1.0)`.
    let mut end_idx = spec.len();
    for (i, c) in spec.char_indices() {
        if matches!(c, '=' | '>' | '<' | '!' | '~' | ';' | '[' | '(' | '@' | ' ') {
            end_idx = i;
            break;
        }
    }

    let name = spec[..end_idx].trim();
    if name.is_empty() {
        None
    } else {
        let marker = spec
            .split_once(';')
            .map(|(_, marker)| marker.trim().to_owned())
            .filter(|marker| !marker.is_empty());
        Some((name.to_owned(), marker))
    }
}

/// Parses a pyproject.toml file and extracts declared project dependencies.
pub fn parse_pyproject(path: &Path) -> Vec<DeclaredDependency> {
    let mut deps = Vec::new();

    let Ok(content) = std::fs::read_to_string(path) else {
        return deps;
    };
    let parsed: Value = match toml::from_str(&content) {
        Ok(value) => value,
        Err(_) => return deps,
    };
    let make_dep = |spec: &str, is_dev, is_optional| {
        let (package_name, marker) = extract_pep508_parts(spec)?;
        Some(DeclaredDependency {
            package_name: package_name.clone(),
            normalized_name: normalize_package_name(&package_name),
            is_dev,
            is_optional,
            marker,
            source: DependencySource::Pyproject,
        })
    };

    if let Some(project) = parsed.get("project") {
        if let Some(dependencies) = project.get("dependencies").and_then(Value::as_array) {
            deps.extend(
                dependencies
                    .iter()
                    .filter_map(Value::as_str)
                    .filter_map(|spec| make_dep(spec, false, false)),
            );
        }
        if let Some(optional) = project
            .get("optional-dependencies")
            .and_then(Value::as_table)
        {
            for reqs in optional.values().filter_map(Value::as_array) {
                deps.extend(
                    reqs.iter()
                        .filter_map(Value::as_str)
                        .filter_map(|spec| make_dep(spec, false, true)),
                );
            }
        }
    }

    if let Some(groups) = parsed.get("dependency-groups").and_then(Value::as_table) {
        for reqs in groups.values().filter_map(Value::as_array) {
            deps.extend(
                reqs.iter()
                    .filter_map(|v| match v {
                        Value::String(s) => Some(s.as_str()),
                        Value::Table(t) => t.get("name").and_then(Value::as_str),
                        _ => None,
                    })
                    .filter_map(|spec| make_dep(spec, true, false)),
            );
        }
    }

    if let Some(tool) = parsed.get("tool").and_then(Value::as_table) {
        if let Some(pdm) = tool.get("pdm").and_then(Value::as_table) {
            if let Some(dev_deps) = pdm.get("dev-dependencies").and_then(Value::as_table) {
                deps.extend(
                    dev_deps
                        .keys()
                        .filter_map(|package_name| make_dep(package_name, true, false)),
                );
            }
        }

        if let Some(poetry) = tool.get("poetry").and_then(Value::as_table) {
            if let Some(poetry_deps) = poetry.get("dependencies").and_then(Value::as_table) {
                deps.extend(
                    poetry_deps
                        .keys()
                        .filter(|package_name| package_name.as_str() != "python")
                        .filter_map(|package_name| make_dep(package_name, false, false)),
                );
            }
            if let Some(dev_deps) = poetry.get("dev-dependencies").and_then(Value::as_table) {
                deps.extend(
                    dev_deps
                        .keys()
                        .filter_map(|package_name| make_dep(package_name, true, false)),
                );
            }
            if let Some(group) = poetry.get("group").and_then(Value::as_table) {
                for grp_val in group.values() {
                    if let Some(grp_deps) = grp_val.get("dependencies").and_then(Value::as_table) {
                        deps.extend(
                            grp_deps
                                .keys()
                                .filter_map(|package_name| make_dep(package_name, true, false)),
                        );
                    }
                }
            }
        }
    }

    deps
}

/// Parses a requirements.txt file and extracts declared dependencies.
pub fn parse_requirements(path: &Path) -> Vec<DeclaredDependency> {
    let mut deps = Vec::new();
    let filename = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    if let Ok(content) = std::fs::read_to_string(path) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('-') {
                continue;
            }

            if let Some((pkg, marker)) = extract_pep508_parts(line) {
                deps.push(DeclaredDependency {
                    package_name: pkg.clone(),
                    normalized_name: normalize_package_name(&pkg),
                    is_dev: filename.contains("dev") || filename.contains("test"),
                    is_optional: false,
                    marker,
                    source: DependencySource::Requirements(filename.clone()),
                });
            }
        }
    }

    deps
}

/// Locates and parses dependency declarations from pyproject.toml or a provided requirements file.
pub fn locate_and_parse_declarations(
    root: &Path,
    req_file_opt: Option<&String>,
) -> Vec<DeclaredDependency> {
    let mut all_deps = Vec::new();

    // First, try pyproject.toml
    let pyproject = root.join("pyproject.toml");
    if pyproject.exists() {
        all_deps.extend(parse_pyproject(&pyproject));
    }

    // Then optionally explicit requirements file, or fallback to auto-discover
    if let Some(req_file) = req_file_opt {
        let req_path = root.join(req_file);
        if req_path.exists() {
            all_deps.extend(parse_requirements(&req_path));
        }
    } else {
        // Auto-discover requirements.txt if it exists
        let req_txt = root.join("requirements.txt");
        if req_txt.exists() {
            all_deps.extend(parse_requirements(&req_txt));
        }
        let dev_req_txt = root.join("requirements-dev.txt");
        if dev_req_txt.exists() {
            all_deps.extend(parse_requirements(&dev_req_txt));
        }
    }

    all_deps
}

#[cfg(test)]
#[path = "declared_tests.rs"]
mod declared_tests;
