//! Pytest plugin visibility and relative-import resolution for fixture lookup.

use super::PytestPluginDeclaration;
use rustc_hash::FxHashSet;
use std::path::Path;

pub(super) fn visible_plugin_modules(
    requester_file: &Path,
    pytest_plugins: &[PytestPluginDeclaration],
) -> FxHashSet<String> {
    let mut visible = FxHashSet::default();
    for plugin in pytest_plugins {
        if plugin_visible_to_request(plugin, requester_file) {
            visible.insert(plugin.plugin_module.clone());
        }
    }
    visible
}

fn plugin_visible_to_request(plugin: &PytestPluginDeclaration, requester_file: &Path) -> bool {
    if plugin
        .declaring_file
        .file_name()
        .is_some_and(|name| name == "conftest.py")
    {
        if let Some(parent) = plugin.declaring_file.parent() {
            return requester_file.starts_with(parent);
        }
        return false;
    }
    requester_file == plugin.declaring_file
}

pub(super) fn path_depth(path: &Path) -> usize {
    path.components().count()
}

pub(super) fn resolve_import_source_module(
    requester_module: &str,
    source_module: &str,
    level: u32,
) -> Option<String> {
    if level == 0 {
        return Some(source_module.to_owned());
    }

    let mut package_parts: Vec<&str> = requester_module.split('.').collect();
    package_parts.pop();

    let ascend = usize::try_from(level - 1).ok()?;
    if ascend > package_parts.len() {
        return None;
    }
    package_parts.truncate(package_parts.len().saturating_sub(ascend));

    if !source_module.is_empty() {
        package_parts.extend(source_module.split('.'));
    }

    Some(package_parts.join("."))
}
