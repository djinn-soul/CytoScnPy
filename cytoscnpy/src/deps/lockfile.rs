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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // Minimal valid uv.lock: version header + 3 packages with deps
    fn uv_lock_sample() -> String {
        let mut s = String::new();
        s.push_str("version = 1\n");
        s.push_str("requires-python = \">=3.10\"\n\n");
        s.push_str("[[package]]\n");
        s.push_str("name = \"requests\"\n");
        s.push_str("version = \"2.31.0\"\n");
        s.push_str("source = { registry = \"https://pypi.org/simple\" }\n");
        s.push_str("dependencies = [\n");
        s.push_str("  { name = \"urllib3\" },\n");
        s.push_str("  { name = \"certifi\" },\n");
        s.push_str("]\n\n");
        s.push_str("[[package]]\n");
        s.push_str("name = \"urllib3\"\n");
        s.push_str("version = \"2.0.0\"\n");
        s.push_str("source = { registry = \"https://pypi.org/simple\" }\n\n");
        s.push_str("[[package]]\n");
        s.push_str("name = \"certifi\"\n");
        s.push_str("version = \"2023.7.22\"\n");
        s.push_str("source = { registry = \"https://pypi.org/simple\" }\n");
        s
    }

    // Real project uv.lock — run with `cargo test -- --include-ignored` to execute.
    // Uses `#[ignore]` because: on a clean CI checkout without a lockfile, the
    // `if path.exists()` guard would make this a silent no-op that always "passes".
    #[test]
    #[ignore = "reads real project uv.lock; run with --include-ignored"]
    fn test_parse_uv_lock_real() {
        // Parse the real project lockfile; it must have many packages
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("uv.lock");
        let content = fs::read_to_string(&path)
            .expect("uv.lock not found — run from the project root with an existing lockfile");
        let nodes = parse_uv_lock(&content);
        assert!(!nodes.is_empty(), "real uv.lock should parse >0 packages");
    }

    #[test]
    fn test_parse_uv_lock() {
        let s = uv_lock_sample();
        let nodes = parse_uv_lock(&s);
        assert_eq!(nodes.len(), 3, "expected 3 packages, got {}", nodes.len());
        let req = nodes.iter().find(|n| n.name == "requests").unwrap();
        assert!(req.deps.contains(&"urllib3".to_owned()));
        assert!(req.deps.contains(&"certifi".to_owned()));
    }

    #[test]
    fn test_parse_poetry_lock() {
        let mut s = String::new();
        s.push_str("[[package]]\n");
        s.push_str("name = \"requests\"\n");
        s.push_str("version = \"2.31.0\"\n\n");
        s.push_str("[package.dependencies]\n");
        s.push_str("urllib3 = \">=1.21.1\"\n");
        s.push_str("certifi = \"*\"\n\n");
        s.push_str("[[package]]\n");
        s.push_str("name = \"urllib3\"\n");
        s.push_str("version = \"2.0.0\"\n\n");
        s.push_str("[[package]]\n");
        s.push_str("name = \"certifi\"\n");
        s.push_str("version = \"2023.7.22\"\n");

        let nodes = parse_poetry_lock(&s);
        assert_eq!(nodes.len(), 3, "expected 3 packages, got {}", nodes.len());
        let req = nodes.iter().find(|n| n.name == "requests").unwrap();
        assert!(req.deps.contains(&"urllib3".to_owned()));
    }

    #[test]
    fn test_transitive_deps() {
        let s = uv_lock_sample();
        let nodes = parse_uv_lock(&s);
        let graph = LockfileGraph::from_nodes(&nodes);
        let transitive = graph.transitive_deps("requests");
        assert!(transitive.contains(&"urllib3".to_owned()));
        assert!(transitive.contains(&"certifi".to_owned()));
        assert!(!transitive.contains(&"requests".to_owned()));
    }

    #[test]
    fn test_reverse_map() {
        let s = uv_lock_sample();
        let nodes = parse_uv_lock(&s);
        let graph = LockfileGraph::from_nodes(&nodes);
        let parents = graph.reverse.get("urllib3").unwrap();
        assert!(parents.contains(&"requests".to_owned()));
    }

    #[test]
    fn test_load_lockfile_graph_from_temp_dir() {
        let tmp = tempdir().unwrap();
        fs::write(tmp.path().join("uv.lock"), uv_lock_sample()).unwrap();
        let graph = load_lockfile_graph(tmp.path());
        assert!(graph.is_some());
        let graph = graph.unwrap();
        assert!(graph.forward.contains_key("requests"));
    }
}
