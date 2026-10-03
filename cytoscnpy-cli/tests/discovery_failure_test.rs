//! A directory that cannot be searched must never produce a successful scan.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::os::unix::{
    fs::{MetadataExt, PermissionsExt},
    process::CommandExt,
};
use std::process::Command;

#[test]
fn main_and_dependency_scans_fail_when_a_directory_is_unreadable() {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755)).unwrap();
    // Copy the CLI so a test running as root can drop child privileges even when
    // the original Cargo target directory is under an inaccessible root home.
    let binary = dir.path().join("scanner");
    fs::copy(env!("CARGO_BIN_EXE_cytoscnpy-cli"), &binary).unwrap();
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
    let root = dir.path().join("project");
    fs::create_dir(&root).unwrap();
    fs::write(
        root.join("pyproject.toml"),
        "[project]\nname='example'\nversion='0.1'\n",
    )
    .unwrap();
    fs::write(root.join("valid.py"), "eval(input())\n").unwrap();
    let locked = root.join("locked");
    fs::create_dir(&locked).unwrap();
    fs::write(locked.join("hidden.py"), "eval(input())\n").unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o0)).unwrap();
    let root_owned = fs::metadata(dir.path()).unwrap().uid() == 0;
    let mut outputs = Vec::new();
    for args in [
        vec![
            root.to_string_lossy().into_owned(),
            "--danger".into(),
            "--json".into(),
        ],
        vec![
            root.to_string_lossy().into_owned(),
            root.join("valid.py").to_string_lossy().into_owned(),
            "--danger".into(),
            "--json".into(),
        ],
        vec![
            "deps".into(),
            root.to_string_lossy().into_owned(),
            "--json".into(),
        ],
    ] {
        let mut command = Command::new(&binary);
        command.args(args);
        if root_owned {
            command.uid(65534).gid(65534);
        }
        outputs.push(command.output().unwrap());
    }
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    for (index, output) in outputs.into_iter().enumerate() {
        assert_eq!(
            output.status.code(),
            Some(1),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        if index == 2 {
            assert_eq!(payload["scan_complete"], false);
            assert!(!payload["scan_errors"].as_array().unwrap().is_empty());
        } else {
            let errors = payload["parse_errors"].as_array().unwrap();
            assert!(!errors.is_empty());
            assert_eq!(
                payload["analysis_summary"]["parse_errors_count"],
                errors.len()
            );
            assert!(!payload["danger"].as_array().unwrap().is_empty());
            assert!(errors
                .iter()
                .any(|error| error["error"].as_str().unwrap().contains("locked")));
        }
    }
}
