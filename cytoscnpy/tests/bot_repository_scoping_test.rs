//! Keep module and commit identities local to each repository.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use cytoscnpy::architecture::resolver::{ModuleResolver, ResolvedTarget};
use cytoscnpy::context::{analyze_context, ContextConfig};
use std::fs;
use std::path::Path;
use std::process::Command;

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn colliding_modules_resolve_absolute_relative_and_short_imports_locally() {
    let temp = tempfile::tempdir().unwrap();
    let roots: Vec<_> = ["a", "b"]
        .iter()
        .map(|name| temp.path().join(name))
        .collect();
    let mut files = Vec::new();
    for root in &roots {
        fs::create_dir_all(root.join("pkg")).unwrap();
        for name in ["service.py", "client.py", "__init__.py"] {
            let file = root.join("pkg").join(name);
            fs::write(&file, "").unwrap();
            files.push(file);
        }
    }
    let resolver = ModuleResolver::build(&files, &roots);
    for root in &roots {
        let source = resolver.file_to_id[&root.join("pkg/client.py")];
        let target = resolver.file_to_id[&root.join("pkg/service.py")];
        for (module, symbol, level) in [
            (Some("pkg.service"), None, 0),
            (Some("service"), None, 0),
            (None, Some("service"), 1),
        ] {
            assert_eq!(
                resolver.resolve_import(source, module, symbol, level),
                Some(ResolvedTarget::Internal(target))
            );
        }
    }
}

#[test]
fn commits_activity_and_hotspots_include_every_repository_once() {
    let temp = tempfile::tempdir().unwrap();
    let mut roots = Vec::new();
    for name in ["a", "b"] {
        let root = temp.path().join(name);
        fs::create_dir(&root).unwrap();
        git(&root, &["init", "-q"]);
        git(&root, &["config", "user.name", "Test"]);
        git(&root, &["config", "user.email", "test@example.com"]);
        fs::write(
            root.join("app.py"),
            "def run(x):\n    if x:\n        return 1\n    return 0\n",
        )
        .unwrap();
        git(&root, &["add", "app.py"]);
        git(&root, &["commit", "-qm", "initial"]);
        roots.push(root);
    }
    // A repeated root must not double-count either files or commits.
    roots.push(roots[0].clone());
    let result = analyze_context(&roots, &[], &ContextConfig::default());
    assert_eq!(result.total_files, 2);
    assert_eq!(result.git_activity.total_commits, 2);
    assert_eq!(result.git_activity.active_files, 2);
    assert_eq!(result.git_activity.hot_files.len(), 2);
    assert_eq!(result.hotspots.len(), 2);
    for root in &roots {
        assert!(result
            .hotspots
            .iter()
            .any(|hot| hot.path == root.join("app.py")));
    }
}
