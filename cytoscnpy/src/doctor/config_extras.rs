use super::types::{ConfigCategory, DetectedConfig};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

pub(super) fn detect_ini_tool_configs(repo_root: &Path) -> Vec<DetectedConfig> {
    let mut detected = Vec::new();
    for file in ["setup.cfg", "tox.ini"] {
        let path = repo_root.join(file);
        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };
        let sections = ini_sections(&contents);
        let mut add = |section: &str, category: ConfigCategory, tool: &str| {
            if sections.contains(section) {
                detected.push(DetectedConfig {
                    category,
                    name: format!("{tool} ({file})"),
                    path: path.clone(),
                    details: Some(format!("Configured in [{section}]")),
                });
            }
        };
        add("flake8", ConfigCategory::Linter, "Flake8");
        add("pylint", ConfigCategory::Linter, "Pylint");
        add("pycodestyle", ConfigCategory::Linter, "pycodestyle");
        add("pep8", ConfigCategory::Formatter, "autopep8");
        add("autopep8", ConfigCategory::Formatter, "autopep8");
        add("isort", ConfigCategory::Formatter, "isort");
        add("yapf", ConfigCategory::Formatter, "YAPF");
        add("mypy", ConfigCategory::TypeChecker, "mypy");
        add("tool:pytest", ConfigCategory::TestFramework, "pytest");
        add("pytest", ConfigCategory::TestFramework, "pytest");
        if sections.contains("tox") || sections.contains("testenv") || sections.contains("tox:tox")
        {
            detected.push(DetectedConfig {
                category: ConfigCategory::BuildScript,
                name: format!("tox ({file})"),
                path: path.clone(),
                details: Some("tox environments configured".to_owned()),
            });
        }
    }
    detected
}

fn ini_sections(contents: &str) -> HashSet<String> {
    contents
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with('#') || line.starts_with(';') {
                return None;
            }
            line.strip_prefix('[')
                .and_then(|section| section.strip_suffix(']'))
                .map(|section| section.trim().to_ascii_lowercase())
        })
        .collect()
}

pub(super) fn detect_named_pylock_files(repo_root: &Path) -> Vec<DetectedConfig> {
    let Ok(entries) = fs::read_dir(repo_root) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?.to_owned();
            let matches = name != "pylock.toml"
                && name.starts_with("pylock.")
                && path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("toml"));
            if matches && path.is_file() {
                Some(DetectedConfig {
                    category: ConfigCategory::Lockfile,
                    name,
                    path,
                    details: None,
                })
            } else {
                None
            }
        })
        .collect()
}

pub(super) fn detect_requirements_files(repo_root: &Path) -> Vec<DetectedConfig> {
    let Ok(entries) = fs::read_dir(repo_root) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?.to_owned();
            let is_requirements = name.starts_with("requirements")
                && path.extension().is_some_and(|ext| {
                    ext.eq_ignore_ascii_case("txt") || ext.eq_ignore_ascii_case("in")
                });
            if is_requirements && path.is_file() {
                Some(DetectedConfig {
                    category: ConfigCategory::DependencyManager,
                    name,
                    path,
                    details: None,
                })
            } else {
                None
            }
        })
        .collect()
}
