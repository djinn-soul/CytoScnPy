use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[path = "pyproject.rs"]
mod pyproject;
#[path = "requirements.rs"]
mod requirements;
pub use pyproject::parse_pyproject;
pub use requirements::{parse_requirements, scan_requirements};

/// Dependencies and errors encountered while reading declaration files.
#[derive(Default)]
pub struct DeclarationScan {
    /// Declarations successfully read from the project metadata.
    pub dependencies: Vec<DeclaredDependency>,
    /// Paths and reasons for incomplete declaration scans.
    pub scan_errors: Vec<crate::analyzer::types::ParseError>,
}

/// Origin of a declared dependency.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DependencySource {
    /// Declared in pyproject.toml.
    Pyproject,
    /// Declared in a requirements.txt file.
    Requirements(String),
    /// Declared in a setuptools file (`setup.py` or `setup.cfg`).
    Setup(String),
}

/// Represents a dependency declared in the project configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredDependency {
    /// The raw package name as it appears in the declaration.
    pub package_name: String,
    /// The normalized package name for comparison.
    pub normalized_name: String,
    /// Whether this is a development dependency.
    pub is_dev: bool,
    /// Whether this dependency came from an optional runtime extra.
    pub is_optional: bool,
    /// PEP 508 environment marker, if present.
    pub marker: Option<String>,
    /// The source file or location of the declaration.
    pub source: DependencySource,
}

/// Normalizes a package name according to PEP 503.
pub fn normalize_package_name(name: &str) -> String {
    name.to_lowercase().replace(['-', '.'], "_")
}

/// Extracts the clean package name from a PEP 508 specification string.
pub fn extract_package_name_from_pep508(spec: &str) -> Option<String> {
    extract_pep508_parts(spec).map(|(name, _)| name)
}

pub(super) fn extract_pep508_parts(spec: &str) -> Option<(String, Option<String>)> {
    let spec = spec.trim();
    if spec.is_empty() || spec.starts_with('#') {
        return None;
    }

    // Skip VCS requirements (git+https://, hg+https://, svn+..., bzr+...)
    // and bare URL requirements (https://, http://) — these have no PyPI package name.
    let lower = spec.to_ascii_lowercase();
    if lower.starts_with("git+")
        || lower.starts_with("hg+")
        || lower.starts_with("svn+")
        || lower.starts_with("bzr+")
        || lower.starts_with("http://")
        || lower.starts_with("https://")
    {
        return None;
    }

    // Extract everything before version specifiers, extras, env markers, or URL separators.
    // Stop chars: `@` handles `pkg @ https://...`, `(` handles `pkg(>=1.0)`.
    let mut end_idx = spec.len();
    for (i, c) in spec.char_indices() {
        if matches!(c, '=' | '>' | '<' | '!' | '~' | ';' | '[' | '(' | '@' | ' ') {
            end_idx = i;
            break;
        }
    }

    let name = spec[..end_idx].trim();
    if name.is_empty() {
        None
    } else {
        let marker = spec
            .split_once(';')
            .map(|(_, marker)| marker.trim().to_owned())
            .filter(|marker| !marker.is_empty());
        Some((name.to_owned(), marker))
    }
}

/// Walks up from `start` to find the directory holding the project's dependency
/// manifest, so that analyzing a subdirectory (`cytoscnpy deps src/`) still sees
/// the declarations. Stops at a `.git` boundary and falls back to `start`.
pub fn find_project_root(start: &Path) -> PathBuf {
    const MANIFESTS: &[&str] = &[
        "pyproject.toml",
        "requirements.txt",
        "setup.cfg",
        "setup.py",
    ];

    let mut current = Some(start);
    while let Some(dir) = current {
        if MANIFESTS.iter().any(|name| dir.join(name).exists()) {
            return dir.to_path_buf();
        }
        if dir != start && dir.join(".git").exists() {
            break;
        }
        current = dir.parent();
    }
    start.to_path_buf()
}

/// Locates and parses dependency declarations from pyproject.toml or a provided requirements file.
pub fn locate_and_parse_declarations(
    root: &Path,
    req_file_opt: Option<&String>,
) -> Vec<DeclaredDependency> {
    let scan = scan_declarations(root, req_file_opt);
    for error in &scan.scan_errors {
        eprintln!(
            "WARNING: Incomplete dependency declarations at {}: {}",
            error.file.display(),
            error.error
        );
    }
    scan.dependencies
}

/// Reads project declarations, preserving missing or unreadable requirements inputs.
/// Missing/malformed pyproject.toml retains the established graceful-degradation behavior.
pub fn scan_declarations(root: &Path, req_file_opt: Option<&String>) -> DeclarationScan {
    let mut scan = DeclarationScan::default();
    let all_deps = &mut scan.dependencies;

    // First, try pyproject.toml
    let pyproject = root.join("pyproject.toml");
    if pyproject.exists() {
        all_deps.extend(parse_pyproject(&pyproject));
    }

    // Pre-PEP-621 setuptools projects declare their dependencies in setup.cfg or
    // setup.py. Both can coexist with a pyproject.toml that only carries build
    // configuration, so they are read regardless of whether pyproject.toml exists.
    let setup_cfg = root.join("setup.cfg");
    if setup_cfg.exists() {
        all_deps.extend(super::setup::parse_setup_cfg(&setup_cfg));
    }
    let setup_py = root.join("setup.py");
    if setup_py.exists() {
        all_deps.extend(super::setup::parse_setup_py(&setup_py));
    }

    // Then optionally explicit requirements file, or fallback to auto-discover
    if let Some(req_file) = req_file_opt {
        let req_path = root.join(req_file);
        let requirements = scan_requirements(&req_path);
        all_deps.extend(requirements.dependencies);
        scan.scan_errors.extend(requirements.scan_errors);
    } else {
        // Auto-discover requirements.txt if it exists
        let req_txt = root.join("requirements.txt");
        if req_txt.exists() {
            let requirements = scan_requirements(&req_txt);
            all_deps.extend(requirements.dependencies);
            scan.scan_errors.extend(requirements.scan_errors);
        }
        let dev_req_txt = root.join("requirements-dev.txt");
        if dev_req_txt.exists() {
            let requirements = scan_requirements(&dev_req_txt);
            all_deps.extend(requirements.dependencies);
            scan.scan_errors.extend(requirements.scan_errors);
        }
    }

    // `-r` includes and overlapping auto-discovered files can yield the same
    // declaration twice; keep the first occurrence of each.
    let mut seen = std::collections::HashSet::new();
    all_deps.retain(|dep| {
        seen.insert((
            dep.normalized_name.clone(),
            dep.is_dev,
            dep.is_optional,
            dep.source.clone(),
        ))
    });

    scan
}

#[cfg(test)]
#[path = "declared_tests.rs"]
mod declared_tests;
