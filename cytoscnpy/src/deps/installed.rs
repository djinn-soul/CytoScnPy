use crate::deps::declared::normalize_package_name;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Represents a package installed in the virtual environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPackage {
    /// The package's distribution name (from METADATA `Name:` field).
    pub name: String,
    /// Normalized package name (PEP 503).
    pub normalized_name: String,
    /// Installed version.
    pub version: String,
    /// Direct runtime dependencies as normalized names.
    pub requires: Vec<String>,
}

/// Parses a single `METADATA` file and extracts the package name, version,
/// and `Requires-Dist` lines.
fn parse_metadata(content: &str) -> Option<InstalledPackage> {
    let mut name: Option<String> = None;
    let mut version: Option<String> = None;
    let mut requires: Vec<String> = Vec::new();

    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("Name:") {
            name = Some(rest.trim().to_owned());
        } else if let Some(rest) = line.strip_prefix("Version:") {
            version = Some(rest.trim().to_owned());
        } else if let Some(rest) = line.strip_prefix("Requires-Dist:") {
            // Requires-Dist: requests (>=2.0); extra == "dev"
            // We only want the base package name, before any specifier or extra marker
            let req = rest.trim();
            let clean = req.split([' ', ';', '(', '>']).next().unwrap_or(req).trim();
            if !clean.is_empty() {
                requires.push(normalize_package_name(clean));
            }
        }
    }

    let name = name?;
    let version = version.unwrap_or_default();
    let normalized_name = normalize_package_name(&name);

    Some(InstalledPackage {
        name,
        normalized_name,
        version,
        requires,
    })
}

/// Finds the `site-packages` directory inside a `.venv`.
/// Handles both Windows (`.venv/Lib/site-packages`) and
/// Unix (`.venv/lib/python*/site-packages`).
fn find_site_packages(venv_root: &Path) -> Option<PathBuf> {
    // Windows layout
    let win = venv_root.join("Lib").join("site-packages");
    if win.is_dir() {
        return Some(win);
    }

    // Unix layout: .venv/lib/python3.X/site-packages
    let lib_dir = venv_root.join("lib");
    if let Ok(entries) = std::fs::read_dir(&lib_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let sp = path.join("site-packages");
                if sp.is_dir() {
                    return Some(sp);
                }
            }
        }
    }

    None
}

/// Scans the virtual environment for installed packages.
///
/// Returns a map from normalized package name to [`InstalledPackage`].
/// Returns an empty map if `venv_root` does not exist or has no `site-packages`.
pub fn scan_installed(venv_root: &Path) -> FxHashMap<String, InstalledPackage> {
    let mut result = FxHashMap::default();

    let Some(site_packages) = find_site_packages(venv_root) else {
        return result;
    };

    let Ok(entries) = std::fs::read_dir(&site_packages) else {
        return result;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        // Look for *.dist-info directories
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_owned(),
            None => continue,
        };
        if !name.ends_with(".dist-info") || !path.is_dir() {
            continue;
        }

        let metadata_path = path.join("METADATA");
        if !metadata_path.exists() {
            continue;
        }

        let Ok(content) = std::fs::read_to_string(&metadata_path) else {
            continue;
        };

        if let Some(pkg) = parse_metadata(&content) {
            result.insert(pkg.normalized_name.clone(), pkg);
        }
    }

    result
}

/// Auto-detect the virtual environment root starting from a project root.
/// Checks `.venv` first, then `venv`, then `env`.
pub fn detect_venv(project_root: &Path) -> Option<PathBuf> {
    for candidate in &[".venv", "venv", "env"] {
        let p = project_root.join(candidate);
        if p.is_dir() && find_site_packages(&p).is_some() {
            return Some(p);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn make_dist_info(site_packages: &Path, name: &str, version: &str, requires: &[&str]) {
        let dir = site_packages.join(format!("{name}-{version}.dist-info"));
        fs::create_dir_all(&dir).unwrap();
        let mut meta = format!("Name: {name}\nVersion: {version}\n");
        for req in requires {
            meta.push_str(&format!("Requires-Dist: {req}\n"));
        }
        fs::write(dir.join("METADATA"), meta).unwrap();
    }

    #[test]
    fn test_scan_installed_windows_layout() {
        let tmp = tempdir().unwrap();
        let venv = tmp.path().join(".venv");
        let site_pkg = venv.join("Lib").join("site-packages");
        fs::create_dir_all(&site_pkg).unwrap();

        make_dist_info(&site_pkg, "requests", "2.31.0", &["urllib3 (>=1.21.1)"]);
        make_dist_info(&site_pkg, "urllib3", "2.0.0", &[]);

        let installed = scan_installed(&venv);
        assert!(installed.contains_key("requests"));
        assert_eq!(installed["requests"].version, "2.31.0");
        assert!(installed["requests"]
            .requires
            .contains(&"urllib3".to_owned()));
        assert!(installed.contains_key("urllib3"));
    }

    #[test]
    fn test_scan_installed_missing_venv() {
        let installed = scan_installed(Path::new("/definitely/does/not/exist"));
        assert!(installed.is_empty());
    }

    #[test]
    fn test_normalize_hyphens_and_dots() {
        assert_eq!(normalize_package_name("scikit-learn"), "scikit_learn");
        assert_eq!(normalize_package_name("zope.interface"), "zope_interface");
    }

    #[test]
    fn test_detect_venv() {
        let tmp = tempdir().unwrap();
        let venv = tmp.path().join(".venv");
        let site_pkg = venv.join("Lib").join("site-packages");
        fs::create_dir_all(&site_pkg).unwrap();

        let detected = detect_venv(tmp.path());
        assert!(detected.is_some());
        assert_eq!(detected.unwrap(), venv);
    }
}
