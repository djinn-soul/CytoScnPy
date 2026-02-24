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
    let mut current = path.to_path_buf();
    if current.is_file() {
        current.pop();
    }

    loop {
        let cytoscnpy_toml = current.join(CONFIG_FILENAME);
        if cytoscnpy_toml.exists() {
            if let Ok(content) = fs::read_to_string(&cytoscnpy_toml) {
                if let Ok(mut config) = toml::from_str::<Config>(&content) {
                    config.config_file_path = Some(cytoscnpy_toml);
                    mark_deprecated_keys_from_content(&mut config, &content, &["cytoscnpy"]);
                    return config;
                }
            }
        }

        let pyproject_toml = current.join(PYPROJECT_FILENAME);
        if pyproject_toml.exists() {
            if let Ok(content) = fs::read_to_string(&pyproject_toml) {
                if let Ok(pyproject) = toml::from_str::<PyProject>(&content) {
                    let mut config = Config {
                        cytoscnpy: pyproject.tool.cytoscnpy,
                        config_file_path: Some(pyproject_toml),
                    };
                    mark_deprecated_keys_from_content(
                        &mut config,
                        &content,
                        &["tool", "cytoscnpy"],
                    );
                    return config;
                }
            }
        }

        if !current.pop() {
            break;
        }
    }

    Config::default()
}

#[cfg(test)]
pub(super) fn mark_deprecated_for_test(config: &mut Config, content: &str, path: &[&str]) {
    mark_deprecated_keys_from_content(config, content, path);
}
