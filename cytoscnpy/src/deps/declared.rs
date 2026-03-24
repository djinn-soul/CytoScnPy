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
    /// Whether this is a development/optional dependency.
    pub is_dev: bool,
    /// The source file or location of the declaration.
    pub source: DependencySource,
}

/// Normalizes a package name according to PEP 503.
pub fn normalize_package_name(name: &str) -> String {
    name.to_lowercase().replace(['-', '.'], "_")
}

/// Extracts the clean package name from a PEP 508 specification string.
pub fn extract_package_name_from_pep508(spec: &str) -> Option<String> {
    let spec = spec.trim();
    if spec.is_empty() || spec.starts_with('#') {
        return None;
    }

    // Extract everything before version specifiers and extras
    let mut end_idx = spec.len();
    for (i, c) in spec.char_indices() {
        if matches!(c, '=' | '>' | '<' | '!' | '~' | ';' | '[' | ' ') {
            end_idx = i;
            break;
        }
    }

    let name = spec[..end_idx].trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_owned())
    }
}

/// Parses a pyproject.toml file and extracts declared project dependencies.
pub fn parse_pyproject(path: &Path) -> Vec<DeclaredDependency> {
    let mut deps = Vec::new();

    if let Ok(content) = std::fs::read_to_string(path) {
        let parsed: Value = match toml::from_str(&content) {
            Ok(v) => v,
            Err(_) => {
                return deps;
            }
        };

        // Parse [project].dependencies
        if let Some(project) = parsed.get("project") {
            if let Some(dependencies) = project.get("dependencies").and_then(Value::as_array) {
                for dep in dependencies.iter().filter_map(Value::as_str) {
                    if let Some(pkg) = extract_package_name_from_pep508(dep) {
                        deps.push(DeclaredDependency {
                            package_name: pkg.clone(),
                            normalized_name: normalize_package_name(&pkg),
                            is_dev: false,
                            source: DependencySource::Pyproject,
                        });
                    }
                }
            }

            // Parse optional-dependencies
            if let Some(optional) = project
                .get("optional-dependencies")
                .and_then(Value::as_table)
            {
                for (_, reqs) in optional {
                    if let Some(req_arr) = reqs.as_array() {
                        for dep in req_arr.iter().filter_map(Value::as_str) {
                            if let Some(pkg) = extract_package_name_from_pep508(dep) {
                                deps.push(DeclaredDependency {
                                    package_name: pkg.clone(),
                                    normalized_name: normalize_package_name(&pkg),
                                    is_dev: true,
                                    source: DependencySource::Pyproject,
                                });
                            }
                        }
                    }
                }
            }
        }

        // tools like poetry and pdm also put dev dependencies elsewhere
        // We can also check [dependency-groups] per PEP 735 (used by uv)
        if let Some(groups) = parsed.get("dependency-groups").and_then(Value::as_table) {
            for (_, reqs) in groups {
                if let Some(req_arr) = reqs.as_array() {
                    for dep in req_arr.iter().filter_map(|v| match v {
                        Value::String(s) => Some(s.as_str()),
                        Value::Table(t) => t.get("include").and_then(Value::as_str),
                        _ => None,
                    }) {
                        if let Some(pkg) = extract_package_name_from_pep508(dep) {
                            deps.push(DeclaredDependency {
                                package_name: pkg.clone(),
                                normalized_name: normalize_package_name(&pkg),
                                is_dev: true,
                                source: DependencySource::Pyproject,
                            });
                        }
                    }
                }
            }
        }

        // Poetry legacy format [tool.poetry.dependencies]
        if let Some(tool) = parsed.get("tool").and_then(Value::as_table) {
            if let Some(poetry) = tool.get("poetry").and_then(Value::as_table) {
                if let Some(poetry_deps) = poetry.get("dependencies").and_then(Value::as_table) {
                    for (pkg, _) in poetry_deps {
                        if pkg != "python" {
                            deps.push(DeclaredDependency {
                                package_name: pkg.clone(),
                                normalized_name: normalize_package_name(pkg),
                                is_dev: false,
                                source: DependencySource::Pyproject,
                            });
                        }
                    }
                }
                if let Some(dev_deps) = poetry.get("dev-dependencies").and_then(Value::as_table) {
                    for (pkg, _) in dev_deps {
                        deps.push(DeclaredDependency {
                            package_name: pkg.clone(),
                            normalized_name: normalize_package_name(pkg),
                            is_dev: true,
                            source: DependencySource::Pyproject,
                        });
                    }
                }
                if let Some(group) = poetry.get("group").and_then(Value::as_table) {
                    for (_, grp_val) in group {
                        if let Some(grp_deps) =
                            grp_val.get("dependencies").and_then(Value::as_table)
                        {
                            for (pkg, _) in grp_deps {
                                deps.push(DeclaredDependency {
                                    package_name: pkg.clone(),
                                    normalized_name: normalize_package_name(pkg),
                                    is_dev: true,
                                    source: DependencySource::Pyproject,
                                });
                            }
                        }
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

            if let Some(pkg) = extract_package_name_from_pep508(line) {
                deps.push(DeclaredDependency {
                    package_name: pkg.clone(),
                    normalized_name: normalize_package_name(&pkg),
                    is_dev: filename.contains("dev") || filename.contains("test"),
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
