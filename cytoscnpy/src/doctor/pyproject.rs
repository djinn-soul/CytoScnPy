use std::fs;
use std::path::Path;
use toml::Value;

use super::types::{ConfigCategory, DetectedConfig, PyProjectInspection};

/// Inspects `pyproject.toml` for tool declarations, dependencies, and build backend.
pub fn inspect_pyproject(repo_root: &Path) -> Option<PyProjectInspection> {
    let pyproject_path = repo_root.join("pyproject.toml");
    if !pyproject_path.exists() {
        return None;
    }

    let content = fs::read_to_string(&pyproject_path).ok()?;
    let value: Value = toml::from_str(&content).ok()?;

    let mut inspection = PyProjectInspection::default();

    if let Some(tool_table) = value.get("tool").and_then(Value::as_table) {
        inspection.has_ruff = tool_table.contains_key("ruff");
        inspection.has_black = tool_table.contains_key("black");
        inspection.has_mypy = tool_table.contains_key("mypy");
        inspection.has_pyright = tool_table.contains_key("pyright");
        inspection.has_pylint = tool_table.contains_key("pylint");
        inspection.has_pytest = tool_table.contains_key("pytest");
        inspection.has_poetry = tool_table.contains_key("poetry");
    }

    if let Some(build_system) = value.get("build-system").and_then(Value::as_table) {
        if let Some(backend) = build_system.get("build-backend").and_then(Value::as_str) {
            inspection.build_backend = Some(backend.to_owned());
            if backend.contains("flit") || backend.contains("hatch") {
                inspection.has_flit_or_hatch = true;
            }
        }
    }

    if let Some(project) = value.get("project").and_then(Value::as_table) {
        if let Some(deps) = project.get("dependencies").and_then(Value::as_array) {
            inspection.dependencies_count = deps.len();
        }
        if let Some(opt_deps) = project
            .get("optional-dependencies")
            .and_then(Value::as_table)
        {
            let count: usize = opt_deps
                .values()
                .filter_map(Value::as_array)
                .map(Vec::len)
                .sum();
            inspection.dev_dependencies_count = count;
        }
    }

    Some(inspection)
}

/// Augments detected configurations with tool configurations found in `pyproject.toml`.
pub fn supplement_configs_with_pyproject(
    configs: &mut Vec<DetectedConfig>,
    pyproject: &PyProjectInspection,
    pyproject_path: &Path,
) {
    if pyproject.has_ruff || pyproject.has_black {
        let name = if pyproject.has_ruff {
            "Ruff (pyproject.toml)"
        } else {
            "Black (pyproject.toml)"
        };
        if !configs
            .iter()
            .any(|c| c.category == ConfigCategory::Formatter)
        {
            configs.push(DetectedConfig {
                category: ConfigCategory::Formatter,
                name: name.to_owned(),
                path: pyproject_path.to_path_buf(),
                details: Some("Configured under [tool.*]".to_owned()),
            });
        }
    }

    if pyproject.has_ruff || pyproject.has_pylint {
        let name = if pyproject.has_ruff {
            "Ruff (pyproject.toml)"
        } else {
            "Pylint (pyproject.toml)"
        };
        if !configs.iter().any(|c| c.category == ConfigCategory::Linter) {
            configs.push(DetectedConfig {
                category: ConfigCategory::Linter,
                name: name.to_owned(),
                path: pyproject_path.to_path_buf(),
                details: Some("Configured under [tool.*]".to_owned()),
            });
        }
    }

    if pyproject.has_mypy || pyproject.has_pyright {
        let name = if pyproject.has_mypy {
            "mypy (pyproject.toml)"
        } else {
            "pyright (pyproject.toml)"
        };
        if !configs
            .iter()
            .any(|c| c.category == ConfigCategory::TypeChecker)
        {
            configs.push(DetectedConfig {
                category: ConfigCategory::TypeChecker,
                name: name.to_owned(),
                path: pyproject_path.to_path_buf(),
                details: Some("Configured under [tool.*]".to_owned()),
            });
        }
    }

    if pyproject.has_pytest
        && !configs
            .iter()
            .any(|c| c.category == ConfigCategory::TestFramework)
    {
        configs.push(DetectedConfig {
            category: ConfigCategory::TestFramework,
            name: "pytest (pyproject.toml)".to_owned(),
            path: pyproject_path.to_path_buf(),
            details: Some("Configured under [tool.pytest]".to_owned()),
        });
    }
}
