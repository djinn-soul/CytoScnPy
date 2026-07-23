use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::constants::{CONFIG_FILENAME, PYPROJECT_FILENAME};

use super::models::{Config, PyProject};

fn mark_deprecated_keys_for_cytoscnpy_table(config: &mut Config, table: &toml::Value) {
    if table.get("complexity").is_some() || table.get("nesting").is_some() {
        config.cytoscnpy.set_uses_deprecated_keys(true);
    }
}

fn value_at_path<'a>(value: &'a toml::Value, path: &[&str]) -> Option<&'a toml::Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn mark_deprecated_keys_from_content(config: &mut Config, content: &str, path: &[&str]) {
    if let Ok(value) = toml::from_str::<toml::Value>(content) {
        if let Some(cytoscnpy_table) = value_at_path(&value, path) {
            mark_deprecated_keys_for_cytoscnpy_table(config, cytoscnpy_table);
        }
    }
}

pub(super) fn load_from_path(path: &Path) -> Config {
    try_load_from_path(path).unwrap_or_default()
}

pub(super) fn try_load_from_path(path: &Path) -> Result<Config> {
    let mut current = path.to_path_buf();
    if current.is_file() {
        current.pop();
    }

    loop {
        let cytoscnpy_toml = current.join(CONFIG_FILENAME);
        if cytoscnpy_toml.exists() {
            return load_cytoscnpy_toml(&cytoscnpy_toml);
        }

        let pyproject_toml = current.join(PYPROJECT_FILENAME);
        if pyproject_toml.exists() {
            if let Some(config) = load_pyproject_toml(&pyproject_toml)? {
                return Ok(config);
            }
        }

        if !current.pop() {
            break;
        }
    }

    Ok(Config::default())
}

fn read_config(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .with_context(|| format!("failed to read configuration file '{}'", path.display()))
}

fn load_cytoscnpy_toml(path: &Path) -> Result<Config> {
    let content = read_config(path)?;
    let mut config = toml::from_str::<Config>(&content)
        .with_context(|| format!("failed to parse configuration file '{}'", path.display()))?;
    config.config_file_path = Some(path.to_path_buf());
    mark_deprecated_keys_from_content(&mut config, &content, &["cytoscnpy"]);
    Ok(config)
}

fn load_pyproject_toml(path: &Path) -> Result<Option<Config>> {
    let content = read_config(path)?;
    let value = toml::from_str::<toml::Value>(&content)
        .with_context(|| format!("failed to parse project file '{}'", path.display()))?;
    if value_at_path(&value, &["tool", "cytoscnpy"]).is_none() {
        return Ok(None);
    }

    let pyproject = toml::from_str::<PyProject>(&content).with_context(|| {
        format!(
            "invalid [tool.cytoscnpy] configuration in '{}'",
            path.display()
        )
    })?;
    let mut config = Config {
        cytoscnpy: pyproject.tool.cytoscnpy,
        config_file_path: Some(path.to_path_buf()),
    };
    mark_deprecated_keys_from_content(&mut config, &content, &["tool", "cytoscnpy"]);
    Ok(Some(config))
}

#[cfg(test)]
pub(super) fn mark_deprecated_for_test(config: &mut Config, content: &str, path: &[&str]) {
    mark_deprecated_keys_from_content(config, content, path);
}
