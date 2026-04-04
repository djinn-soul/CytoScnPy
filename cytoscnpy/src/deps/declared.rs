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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_normalize_package_name() {
        assert_eq!(normalize_package_name("Requests"), "requests");
        assert_eq!(normalize_package_name("scikit-learn"), "scikit_learn");
        assert_eq!(normalize_package_name("Flask.restful"), "flask_restful");
    }

    #[test]
    fn test_extract_package_name_from_pep508() {
        // Basic cases
        assert_eq!(
            extract_package_name_from_pep508("requests>=2.28.0").unwrap(),
            "requests"
        );
        assert_eq!(
            extract_package_name_from_pep508("scikit-learn[alldeps]").unwrap(),
            "scikit-learn"
        );
        assert_eq!(
            extract_package_name_from_pep508("numpy ; python_version < '3.9'").unwrap(),
            "numpy"
        );
        // Parenthesized specifier: pkg(>=1.0) — no space before (
        assert_eq!(
            extract_package_name_from_pep508("requests(>=2.28,<3)").unwrap(),
            "requests"
        );
        // URL requirement: pkg @ https://...
        assert_eq!(
            extract_package_name_from_pep508("mylib @ https://example.com/mylib.tar.gz").unwrap(),
            "mylib"
        );
        // VCS requirements — must return None (no PyPI package name)
        assert!(extract_package_name_from_pep508("git+https://github.com/user/repo.git").is_none());
        assert!(extract_package_name_from_pep508("hg+https://bitbucket.org/user/repo").is_none());
        assert!(extract_package_name_from_pep508("svn+https://svn.example.com/repo").is_none());
        // Direct URL — must return None
        assert!(extract_package_name_from_pep508("https://example.com/pkg.tar.gz").is_none());
        // Empty / comment — must return None
        assert!(extract_package_name_from_pep508("").is_none());
        assert!(extract_package_name_from_pep508("# just a comment").is_none());
    }

    #[test]
    fn test_parse_requirements_edge_cases() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let req_path = dir.path().join("requirements.txt");
        fs::write(
            &req_path,
            concat!(
                "requests>=2.28.0\n",
                "numpy; python_version>=\"3.8\"\n", // env marker
                "mylib @ https://example.com/mylib.tar.gz\n", // URL requirement
                "git+https://github.com/user/repo.git\n", // VCS — skip
                "https://example.com/pkg.tar.gz\n", // bare URL — skip
                "-r other-requirements.txt\n",      // flag — skip
                "# a comment\n",                    // comment — skip
                "\n",                               // blank line — skip
            ),
        )?;

        let deps = parse_requirements(&req_path);
        let names: Vec<&str> = deps.iter().map(|d| d.package_name.as_str()).collect();

        assert!(names.contains(&"requests"), "requests missing");
        assert!(names.contains(&"numpy"), "numpy missing");
        assert!(names.contains(&"mylib"), "mylib (@ URL) missing");
        // VCS and bare URLs must be skipped
        assert!(
            !names.iter().any(|n| n.starts_with("git")),
            "VCS should be skipped"
        );
        assert!(
            !names.iter().any(|n| n.starts_with("https")),
            "bare URL should be skipped"
        );
        assert_eq!(deps.len(), 3, "expected exactly 3 deps, got {}", deps.len());
        Ok(())
    }

    #[test]
    fn test_parse_pyproject_basics() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let pyproject_path = dir.path().join("pyproject.toml");
        fs::write(
            &pyproject_path,
            r#"
[project]
dependencies = ["requests", "flask>=2.0"]
[project.optional-dependencies]
test = ["pytest"]
"#,
        )?;

        let deps = parse_pyproject(&pyproject_path);
        let names: Vec<String> = deps.iter().map(|d| d.package_name.clone()).collect();
        assert!(names.contains(&"requests".to_owned()));
        assert!(names.contains(&"flask".to_owned()));
        assert!(names.contains(&"pytest".to_owned()));
        Ok(())
    }

    #[test]
    fn test_parse_requirements_basics() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let req_path = dir.path().join("requirements.txt");
        fs::write(&req_path, "requests==2.28.1\n# comment\nnumpy\n")?;

        let deps = parse_requirements(&req_path);
        let names: Vec<String> = deps.iter().map(|d| d.package_name.clone()).collect();
        assert!(names.contains(&"requests".to_owned()));
        assert!(names.contains(&"numpy".to_owned()));
        Ok(())
    }
}
