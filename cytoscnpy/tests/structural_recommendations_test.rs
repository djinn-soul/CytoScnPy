//! Integration tests for top-level directory breakdowns, function collision disambiguation,
//! and stable library extraction recommendations.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::Cursor;
use std::process::Command;
use tempfile::TempDir;

use cytoscnpy::entry_point;

#[test]
fn test_doctor_and_score_top_level_directories() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let src_dir = root.join("src");
    let tests_dir = root.join("tests");
    let docs_dir = root.join("docs");

    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&tests_dir).unwrap();
    fs::create_dir_all(&docs_dir).unwrap();

    fs::write(src_dir.join("main.py"), "def start(): pass\n").unwrap();
    fs::write(tests_dir.join("test_main.py"), "def test_start(): pass\n").unwrap();
    fs::write(docs_dir.join("conf.py"), "# Sphinx configuration\n").unwrap();

    // 1. Doctor command JSON output
    let mut out_doctor_json = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
            root.to_string_lossy().into_owned(),
            "--json".to_owned(),
        ],
        &mut out_doctor_json,
    )
    .unwrap();
    assert_eq!(code, 0);

    let doc_output = String::from_utf8(out_doctor_json.into_inner()).unwrap();
    let doc_json: serde_json::Value = serde_json::from_str(&doc_output).unwrap();
    let top_dirs: Vec<String> = doc_json["health"][0]["structure"]["top_level_directories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();

    assert!(top_dirs.contains(&"docs".to_owned()));
    assert!(top_dirs.contains(&"src".to_owned()));
    assert!(top_dirs.contains(&"tests".to_owned()));
    assert_eq!(
        doc_json["health"][0]["structure"]["top_level_directory_count"],
        3
    );

    // 2. Doctor terminal table output
    let mut out_doctor_txt = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
            root.to_string_lossy().into_owned(),
            "--verbose".to_owned(),
        ],
        &mut out_doctor_txt,
    )
    .unwrap();
    assert_eq!(code, 0);
    let doc_txt = String::from_utf8(out_doctor_txt.into_inner()).unwrap();
    assert!(doc_txt.contains("Top-Level Dirs"));
    assert!(doc_txt.contains("docs"));
    assert!(doc_txt.contains("src"));
    assert!(doc_txt.contains("tests"));

    // 3. Score command JSON output
    let mut out_score_json = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
            root.to_string_lossy().into_owned(),
            "--json".to_owned(),
        ],
        &mut out_score_json,
    )
    .unwrap();
    assert_eq!(code, 0);

    let score_output = String::from_utf8(out_score_json.into_inner()).unwrap();
    let score_json: serde_json::Value = serde_json::from_str(&score_output).unwrap();
    let summary_top_dirs: Vec<String> = score_json["scoring"]["summary"]["top_level_directories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();

    assert_eq!(summary_top_dirs, top_dirs);
    assert_eq!(
        score_json["scoring"]["summary"]["top_level_directory_count"],
        3
    );
}

#[test]
fn test_disambiguate_colliding_functions_recommendation() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Two files do not meet the collision threshold.
    for dir in ["pkg_a", "pkg_b"] {
        let dir_path = root.join(dir);
        fs::create_dir_all(&dir_path).unwrap();
        fs::write(
            dir_path.join("task.py"),
            "def execute_task():\n    return 'done'\n",
        )
        .unwrap();
    }

    let mut out_two = Cursor::new(Vec::new());
    entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
            root.to_string_lossy().into_owned(),
            "--json".to_owned(),
        ],
        &mut out_two,
    )
    .unwrap();
    let two: serde_json::Value = serde_json::from_slice(out_two.get_ref()).unwrap();
    assert!(two["recommendations"]
        .as_array()
        .unwrap()
        .iter()
        .all(|rec| rec["id"] != "disambiguate-function-names"));

    let third = root.join("pkg_c");
    fs::create_dir_all(&third).unwrap();
    fs::write(
        third.join("task.py"),
        "def execute_task():\n    return 'done'\n",
    )
    .unwrap();

    let mut out = Cursor::new(Vec::new());
    let code = entry_point::run_with_args_to(
        vec![
            "deslop".to_owned(),
            root.to_string_lossy().into_owned(),
            "--format".to_owned(),
            "llm".to_owned(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(code, 0);
    let output_str = String::from_utf8(out.into_inner()).unwrap();

    assert!(
        output_str.contains("disambiguate-function-names")
            || output_str.contains("Disambiguate 1 colliding function name(s)"),
        "Expected disambiguate function names recommendation in:\n{output_str}"
    );
}

#[test]
fn test_extract_stable_library_recommendation() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    let lib_dir = root.join("lib");
    fs::create_dir_all(&lib_dir).unwrap();
    let frozen_path = lib_dir.join("legacy_engine.py");
    let active_path = root.join("active.py");
    fs::write(
        &frozen_path,
        format!(
            "def stable():\n    return 1\n{}",
            format!("#{}\n", "x".repeat(110)).repeat(600)
        ),
    )
    .unwrap();
    fs::write(
        &active_path,
        format!(
            "def active():\n    return 1\n{}",
            format!("#{}\n", "y".repeat(110)).repeat(5_000)
        ),
    )
    .unwrap();

    let score = || {
        let mut out = Cursor::new(Vec::new());
        let code = entry_point::run_with_args_to(
            vec![
                "deslop".to_owned(),
                root.to_string_lossy().into_owned(),
                "--json".to_owned(),
                "--context-budget".to_owned(),
                "1000".to_owned(),
            ],
            &mut out,
        )
        .unwrap();
        assert_eq!(code, 0);
        serde_json::from_slice::<serde_json::Value>(out.get_ref()).unwrap()
    };
    let has_recommendation = |result: &serde_json::Value| {
        result["recommendations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|rec| rec["id"] == "extract-stable-library")
            .cloned()
    };

    assert!(
        has_recommendation(&score()).is_none(),
        "Git history is required"
    );

    assert!(Command::new("git")
        .args(["init", "-q"])
        .current_dir(root)
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .args(["add", "."])
        .current_dir(root)
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-qm",
            "old code"
        ])
        .env("GIT_AUTHOR_DATE", "2025-01-01T12:00:00+00:00")
        .env("GIT_COMMITTER_DATE", "2025-01-01T12:00:00+00:00")
        .current_dir(root)
        .status()
        .unwrap()
        .success());
    fs::write(
        &active_path,
        format!("{}\n# updated\n", fs::read_to_string(&active_path).unwrap()),
    )
    .unwrap();
    assert!(Command::new("git")
        .args(["add", "active.py"])
        .current_dir(root)
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-qm",
            "active update"
        ])
        .current_dir(root)
        .status()
        .unwrap()
        .success());

    let result = score();
    let recommendation =
        has_recommendation(&result).expect("expected stable-library recommendation");
    assert_eq!(recommendation["affected_files"][0], "lib/legacy_engine.py");
    assert!(recommendation["estimated_reduction"].as_u64().unwrap() > 0);

    // Keeping the file on disk but removing it from Git must not classify it as mature.
    assert!(Command::new("git")
        .args(["rm", "--cached", "-q", "lib/legacy_engine.py"])
        .current_dir(root)
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-qm",
            "untrack old code",
        ])
        .current_dir(root)
        .status()
        .unwrap()
        .success());
    assert!(has_recommendation(&score()).is_none());
}
