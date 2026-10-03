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
        inspection.formatters = [
            ("ruff", "Ruff"),
            ("black", "Black"),
            ("isort", "isort"),
            ("yapf", "YAPF"),
            ("autopep8", "autopep8"),
        ]
        .into_iter()
        .filter(|(key, _)| tool_table.contains_key(*key))
        .map(|(_, name)| name.to_owned())
        .collect();
        inspection.linters = [
            ("ruff", "Ruff"),
            ("flake8", "Flake8"),
            ("pylint", "Pylint"),
            ("pycodestyle", "pycodestyle"),
        ]
        .into_iter()
        .filter(|(key, _)| tool_table.contains_key(*key))
        .map(|(_, name)| name.to_owned())
        .collect();
        inspection.task_runners = ["tox", "nox"]
            .into_iter()
            .filter(|key| tool_table.contains_key(*key))
            .map(str::to_owned)
            .collect();
        inspection.has_mypy = tool_table.contains_key("mypy");
        inspection.has_pyright = tool_table.contains_key("pyright");
        inspection.type_checkers = [
            ("mypy", "mypy"),
            ("pyright", "Pyright"),
            ("basedpyright", "BasedPyright"),
            ("pyre", "Pyre"),
            ("pytype", "pytype"),
            ("ty", "ty"),
            ("pyrefly", "Pyrefly"),
        ]
        .into_iter()
        .filter(|(key, _)| tool_table.contains_key(*key))
        .map(|(_, name)| name.to_owned())
        .collect();
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
    append_pyproject_tools(
        configs,
        &pyproject.formatters,
        ConfigCategory::Formatter,
        pyproject_path,
    );
    append_pyproject_tools(
        configs,
        &pyproject.linters,
        ConfigCategory::Linter,
        pyproject_path,
    );
    append_pyproject_tools(
        configs,
        &pyproject.type_checkers,
        ConfigCategory::TypeChecker,
        pyproject_path,
    );
    append_pyproject_tools(
        configs,
        &pyproject.task_runners,
        ConfigCategory::BuildScript,
        pyproject_path,
    );

    if pyproject.has_pytest {
        configs.push(DetectedConfig {
            category: ConfigCategory::TestFramework,
            name: "pytest (pyproject.toml)".to_owned(),
            path: pyproject_path.to_path_buf(),
            details: Some("Configured under [tool.pytest]".to_owned()),
        });
    }
}

fn append_pyproject_tools(
    configs: &mut Vec<DetectedConfig>,
    tools: &[String],
    category: ConfigCategory,
    pyproject_path: &Path,
) {
    for tool in tools {
        configs.push(DetectedConfig {
            category,
            name: format!("{tool} (pyproject.toml)"),
            path: pyproject_path.to_path_buf(),
            details: Some(format!(
                "Configured under [tool.{}]",
                tool.to_ascii_lowercase()
            )),
        });
    }
}
