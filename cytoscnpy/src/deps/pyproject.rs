use super::{extract_pep508_parts, normalize_package_name, DeclaredDependency, DependencySource};
use std::path::Path;
use toml::Value;

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
                        .values()
                        .filter_map(Value::as_array)
                        .flatten()
                        .filter_map(Value::as_str)
                        .filter_map(|spec| make_dep(spec, true, false)),
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
