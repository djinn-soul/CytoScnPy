use crate::deps::declared::normalize_package_name;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Represents a node in the parsed lockfile dependency graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockNode {
    /// The package's normalized distribution name.
    pub name: String,
    /// Package version.
    pub version: String,
    /// Normalized names of direct runtime dependencies.
    pub deps: Vec<String>,
}

/// Bidirectional dependency graph sourced from a lockfile.
#[derive(Debug, Default)]
pub struct LockfileGraph {
    /// Forward map: package → direct dependencies.
    pub forward: FxHashMap<String, Vec<String>>,
    /// Reverse map: package → packages that depend on it.
    pub reverse: FxHashMap<String, Vec<String>>,
}

impl LockfileGraph {
    /// Build the graph from a list of lock nodes.
    pub fn from_nodes(nodes: &[LockNode]) -> Self {
        let mut forward: FxHashMap<String, Vec<String>> = FxHashMap::default();
        let mut reverse: FxHashMap<String, Vec<String>> = FxHashMap::default();

        for node in nodes {
            forward
                .entry(node.name.clone())
                .or_default()
                .extend(node.deps.iter().cloned());
            for dep in &node.deps {
                reverse
                    .entry(dep.clone())
                    .or_default()
                    .push(node.name.clone());
            }
        }

        Self { forward, reverse }
    }

    /// Returns the set of packages transitively reachable from `root`.
    pub fn transitive_deps(&self, root: &str) -> Vec<String> {
        let mut visited = vec![];
        let mut queue = vec![root.to_owned()];
        let mut seen = rustc_hash::FxHashSet::default();

        while let Some(pkg) = queue.pop() {
            if !seen.insert(pkg.clone()) {
                continue;
            }
            if pkg != root {
                visited.push(pkg.clone());
            }
            if let Some(children) = self.forward.get(&pkg) {
                queue.extend(children.iter().cloned());
            }
        }
        visited.sort();
        visited
    }
}

// ──────────────────────────────────────────────────────────────
// uv.lock parser
// ──────────────────────────────────────────────────────────────

/// Parse a `uv.lock` file (TOML-based format used by uv).
///
/// Each package block looks like:
/// ```toml
/// [[package]]
/// name = "requests"
/// version = "2.31.0"
/// dependencies = [
///   { name = "urllib3" },
/// ]
/// ```
pub fn parse_uv_lock(content: &str) -> Vec<LockNode> {
    // uv.lock has a preamble (version = 1, requires-python) before [[package]] entries.
    // `toml 0.9` rejects bare integer values at the document root, so strip everything
    // before the first [[package]] marker before handing off to the TOML parser.
    let toml_input = match content.find("[[package]]") {
        Some(idx) => &content[idx..],
        None => content,
    };
    let Ok(value) = toml::from_str::<toml::Value>(toml_input) else {
        return vec![];
    };

    let Some(packages) = value.get("package").and_then(|v| v.as_array()) else {
        return vec![];
    };

    let mut nodes = Vec::new();
    for pkg in packages {
        let name = pkg
            .get("name")
            .and_then(|v| v.as_str())
            .map(normalize_package_name);
        let version = pkg
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();

        let Some(name) = name else { continue };

        let deps: Vec<String> = pkg
            .get("dependencies")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|d| d.get("name").and_then(|v| v.as_str()))
                    .map(normalize_package_name)
                    .collect()
            })
            .unwrap_or_default();

        nodes.push(LockNode {
            name,
            version,
            deps,
        });
    }
    nodes
}

// ──────────────────────────────────────────────────────────────
// poetry.lock parser
// ──────────────────────────────────────────────────────────────

/// Parse a `poetry.lock` file (TOML-based format used by Poetry).
///
/// Each package block looks like:
/// ```toml
/// [[package]]
/// name = "requests"
/// version = "2.31.0"
/// [package.dependencies]
/// urllib3 = ">=1.21.1"
/// ```
pub fn parse_poetry_lock(content: &str) -> Vec<LockNode> {
    let Ok(value) = toml::from_str::<toml::Value>(content) else {
        return vec![];
    };

    let Some(packages) = value.get("package").and_then(|v| v.as_array()) else {
        return vec![];
    };

    let mut nodes = Vec::new();
    for pkg in packages {
        let name = pkg
            .get("name")
            .and_then(|v| v.as_str())
            .map(normalize_package_name);
        let version = pkg
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();

        let Some(name) = name else { continue };

        let deps: Vec<String> = pkg
            .get("dependencies")
            .and_then(|v| v.as_table())
            .map(|t| t.keys().map(|k| normalize_package_name(k)).collect())
            .unwrap_or_default();

        nodes.push(LockNode {
            name,
            version,
            deps,
        });
    }
    nodes
}

/// Auto-detect and parse whichever lockfile exists in `project_root`.
/// Returns `None` if neither `uv.lock` nor `poetry.lock` is present.
pub fn load_lockfile_graph(project_root: &Path) -> Option<LockfileGraph> {
    let uv_path = project_root.join("uv.lock");
    if uv_path.exists() {
        let content = std::fs::read_to_string(&uv_path).ok()?;
        let nodes = parse_uv_lock(&content);
        if !nodes.is_empty() {
            return Some(LockfileGraph::from_nodes(&nodes));
        }
    }

    let poetry_path = project_root.join("poetry.lock");
    if poetry_path.exists() {
        let content = std::fs::read_to_string(&poetry_path).ok()?;
        let nodes = parse_poetry_lock(&content);
        if !nodes.is_empty() {
            return Some(LockfileGraph::from_nodes(&nodes));
        }
    }

    None
}

/// Parse the exact lockfile path supplied by the user.
pub fn load_lockfile_graph_at(lockfile_path: &Path) -> Option<LockfileGraph> {
    let content = std::fs::read_to_string(lockfile_path).ok()?;
    let filename = lockfile_path.file_name()?.to_string_lossy();
    let nodes = match filename.as_ref() {
        "uv.lock" => parse_uv_lock(&content),
        "poetry.lock" => parse_poetry_lock(&content),
        _ => return None,
    };
    (!nodes.is_empty()).then(|| LockfileGraph::from_nodes(&nodes))
}

#[cfg(test)]
#[path = "lockfile_tests.rs"]
mod tests;
