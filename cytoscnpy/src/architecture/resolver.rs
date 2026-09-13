//! Resolves Python source files and import statements to canonical module identities.

use super::types::ModuleNode;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Outcome of attempting to resolve an import in the context of the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedTarget {
    /// Import resolves to an internal project module ID.
    Internal(usize),
    /// Import refers to an external third-party or standard library package.
    External(String),
}

/// Bidirectional index mapping file paths and module names.
#[derive(Debug, Default)]
pub struct ModuleResolver {
    /// Maps canonical module name to node index.
    pub module_to_id: HashMap<String, usize>,
    /// Maps file path (canonical/normalized) to node index.
    pub file_to_id: HashMap<PathBuf, usize>,
    /// All indexed module nodes.
    pub nodes: Vec<ModuleNode>,
}

impl ModuleResolver {
    /// Builds an index of module nodes from a list of discovered Python files.
    pub fn build(files: &[PathBuf], roots: &[PathBuf]) -> Self {
        let mut resolver = Self::default();

        for file in files {
            let relative = find_best_relative_path(file, roots);
            let (module_name, is_init) = path_to_module_name(&relative);
            let package_group = extract_package_group(&module_name);
            let line_count = count_lines(file);

            let id = resolver.nodes.len();
            let node = ModuleNode {
                id,
                name: module_name.clone(),
                file_path: file.clone(),
                relative_path: relative.to_string_lossy().to_string(),
                line_count,
                is_init,
                package_group,
            };

            resolver.file_to_id.insert(file.clone(), id);
            resolver.module_to_id.insert(module_name.clone(), id);

            // Also index with src/ prefix stripped if applicable
            if let Some(stripped) = module_name.strip_prefix("src.") {
                resolver
                    .module_to_id
                    .entry(stripped.to_owned())
                    .or_insert(id);
            }
            if let Some(stripped) = module_name.strip_prefix("python.") {
                resolver
                    .module_to_id
                    .entry(stripped.to_owned())
                    .or_insert(id);
            }

            resolver.nodes.push(node);
        }

        resolver
    }

    /// Resolves an import statement originating from a source module.
    pub fn resolve_import(
        &self,
        source_id: usize,
        module: Option<&str>,
        imported_symbol: Option<&str>,
        level: u32,
    ) -> Option<ResolvedTarget> {
        let source_node = self.nodes.get(source_id)?;

        if level > 0 {
            self.resolve_relative(source_node, module, imported_symbol, level)
        } else {
            self.resolve_absolute(module, imported_symbol)
        }
    }

    fn resolve_relative(
        &self,
        source_node: &ModuleNode,
        module: Option<&str>,
        imported_symbol: Option<&str>,
        level: u32,
    ) -> Option<ResolvedTarget> {
        let source_parts: Vec<&str> = source_node.name.split('.').collect();
        let base_len = if source_node.is_init {
            source_parts.len()
        } else {
            source_parts.len().saturating_sub(1)
        };

        let retain_len = base_len.checked_sub(level as usize - 1)?;

        let mut target_parts: Vec<String> = source_parts[..retain_len]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();

        if let Some(mod_name) = module {
            for part in mod_name.split('.') {
                if !part.is_empty() {
                    target_parts.push(part.to_owned());
                }
            }
        }

        let base_module = target_parts.join(".");

        // Case 1: `from . import submodule` where submodule is an actual module
        if let Some(sym) = imported_symbol {
            let candidate_with_sym = if base_module.is_empty() {
                sym.to_owned()
            } else {
                format!("{base_module}.{sym}")
            };
            if let Some(&id) = self.module_to_id.get(&candidate_with_sym) {
                return Some(ResolvedTarget::Internal(id));
            }
        }

        // Case 2: base_module itself is a known module
        if !base_module.is_empty() {
            if let Some(&id) = self.module_to_id.get(&base_module) {
                return Some(ResolvedTarget::Internal(id));
            }
        }

        None
    }

    fn resolve_absolute(
        &self,
        module: Option<&str>,
        imported_symbol: Option<&str>,
    ) -> Option<ResolvedTarget> {
        let mod_str = module?;
        let trimmed = mod_str.trim();
        if trimmed.is_empty() {
            return None;
        }

        // Case 1: `from a.b import c` where `a.b.c` is a module
        if let Some(sym) = imported_symbol {
            let combined = format!("{trimmed}.{sym}");
            if let Some(&id) = self.find_module_id(&combined) {
                return Some(ResolvedTarget::Internal(id));
            }
        }

        // Case 2: `mod_str` directly matches an internal module
        if let Some(&id) = self.find_module_id(trimmed) {
            return Some(ResolvedTarget::Internal(id));
        }

        // Case 3: Parent prefix matches an internal module
        let parts: Vec<&str> = trimmed.split('.').collect();
        for i in (1..parts.len()).rev() {
            let prefix = parts[..i].join(".");
            if let Some(&id) = self.find_module_id(&prefix) {
                return Some(ResolvedTarget::Internal(id));
            }
        }

        // Not internal: extract top-level package name
        let top_level = parts.first().unwrap_or(&trimmed);
        Some(ResolvedTarget::External((*top_level).to_owned()))
    }

    fn find_module_id(&self, name: &str) -> Option<&usize> {
        self.module_to_id.get(name)
    }
}

fn find_best_relative_path(file: &Path, roots: &[PathBuf]) -> PathBuf {
    for root in roots {
        if let Ok(rel) = file.strip_prefix(root) {
            return rel.to_path_buf();
        }
    }
    file.to_path_buf()
}

fn path_to_module_name(path: &Path) -> (String, bool) {
    let mut parts: Vec<String> = Vec::new();
    let mut is_init = false;

    for comp in path.components() {
        let s = comp.as_os_str().to_string_lossy();
        if s.ends_with(".py") {
            let stem = s.trim_end_matches(".py");
            if stem == "__init__" {
                is_init = true;
            } else {
                parts.push(stem.to_owned());
            }
        } else {
            parts.push(s.into_owned());
        }
    }

    let name = if parts.is_empty() {
        "__main__".to_owned()
    } else {
        parts.join(".")
    };

    (name, is_init)
}

fn extract_package_group(module_name: &str) -> String {
    let first = module_name.split('.').next().unwrap_or(module_name);
    if first.is_empty() {
        "<root>".to_owned()
    } else {
        first.to_owned()
    }
}

fn count_lines(path: &Path) -> usize {
    std::fs::read_to_string(path)
        .map(|s| s.lines().count())
        .unwrap_or(0)
}
