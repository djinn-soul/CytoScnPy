use std::path::{Path, PathBuf};

use super::estimator::{
    compute_token_budget, estimate_discovery_tokens, estimate_reading_tokens,
    estimate_tokens_for_content, estimate_tracing_tokens,
};
use super::git_scanner::{
    auto_window_days, format_window_label, parse_file_frequency, scan_git_activity, ScannedFileInfo,
};
use super::hotspots::{classify_hotspot_risk, has_severe_hotspots};
use super::types::{ContextStatusVerdict, HotspotFile, HotspotRiskLevel};

#[test]
fn test_parse_git_log_output() {
    let log = "src/main.py\n\nsrc/utils.py\nsrc/main.py\nsrc/core.py\nsrc/main.py\n";
    let freq = parse_file_frequency(log);
    assert_eq!(freq.get(Path::new("src/main.py")), Some(&3));
    assert_eq!(freq.get(Path::new("src/utils.py")), Some(&1));
    assert_eq!(freq.get(Path::new("src/core.py")), Some(&1));
    assert_eq!(freq.len(), 3);
}

#[test]
fn test_parse_empty_git_log() {
    let freq = parse_file_frequency("");
    assert!(freq.is_empty());
}

#[test]
fn test_auto_window_scaling() {
    // 2-week-old repo -> 14 / 3 = 4, clamped to min 7
    assert_eq!(auto_window_days(14, 30), 7);
    // 1-month repo -> 30 / 3 = 10
    assert_eq!(auto_window_days(30, 30), 10);
    // 90-day repo -> 90 / 3 = 30, capped at max 30
    assert_eq!(auto_window_days(90, 30), 30);
    // 1-year repo -> 365 / 3 = 121, capped at max 30
    assert_eq!(auto_window_days(365, 30), 30);
    // 1-year repo with --git-months 6 (180 days) -> 121
    assert_eq!(auto_window_days(365, 180), 121);
}

#[test]
fn test_format_window_label() {
    assert_eq!(format_window_label(7), "7 days");
    assert_eq!(format_window_label(13), "13 days");
    assert_eq!(format_window_label(14), "2 weeks");
    assert_eq!(format_window_label(30), "4 weeks");
    assert_eq!(format_window_label(60), "2 months");
}

#[test]
fn test_classify_hotspot_risk() {
    assert_eq!(classify_hotspot_risk(10, 20), HotspotRiskLevel::Critical);
    assert_eq!(classify_hotspot_risk(5, 10), HotspotRiskLevel::High);
    assert_eq!(classify_hotspot_risk(3, 6), HotspotRiskLevel::Medium);
    assert_eq!(classify_hotspot_risk(1, 2), HotspotRiskLevel::Low);
}

#[test]
fn test_has_severe_hotspots() {
    let benign = vec![HotspotFile {
        path: PathBuf::from("a.py"),
        display_path: "a.py".to_owned(),
        commit_count: 1,
        cyclomatic_complexity: 2,
        risk_score: 2,
        risk_level: HotspotRiskLevel::Low,
    }];
    assert!(!has_severe_hotspots(&benign));

    let severe = vec![HotspotFile {
        path: PathBuf::from("critical.py"),
        display_path: "critical.py".to_owned(),
        commit_count: 15,
        cyclomatic_complexity: 30,
        risk_score: 450,
        risk_level: HotspotRiskLevel::Critical,
    }];
    assert!(has_severe_hotspots(&severe));
}

#[test]
fn test_estimate_tokens_for_content() {
    assert_eq!(estimate_tokens_for_content(""), 0);

    let python_code = "def process(items):\n    return [item.upper() for item in items]\n";
    let tokens = estimate_tokens_for_content(python_code);
    assert!(
        (10..=35).contains(&tokens),
        "Expected between 10 and 35 tokens, got {tokens}"
    );

    let symbols = "()[]{}:,.<>=+-*/";
    let symbol_tokens = estimate_tokens_for_content(symbols);
    assert_eq!(symbol_tokens, symbols.len());
}

#[test]
fn test_token_budget_computation() {
    let budget = compute_token_budget(10, 1000, 4.0, 2.0, 1, 176_000);
    assert!(budget.total_navigation_tokens > 0);
    assert!(budget.total_navigation_tokens < budget.usable_context);
    assert_eq!(budget.status_verdict, ContextStatusVerdict::Generous);
    assert_eq!(
        budget.remaining_tokens,
        budget.usable_context - budget.total_navigation_tokens
    );
}

#[test]
fn test_token_budget_exhausted() {
    // When navigation exceeds usable capacity
    let budget = compute_token_budget(1000, 500_000, 25.0, 15.0, 50, 2_000);
    assert_eq!(budget.remaining_tokens, 0);
    assert_eq!(budget.status_verdict, ContextStatusVerdict::Exhausted);
}

#[test]
fn test_scan_git_activity_no_git() {
    let files = vec![
        ScannedFileInfo {
            path: PathBuf::from("a.py"),
            lines: 50,
            bytes: 1200,
        },
        ScannedFileInfo {
            path: PathBuf::from("b.py"),
            lines: 100,
            bytes: 3000,
        },
    ];

    let (activity, commit_map) =
        scan_git_activity(Path::new("/nonexistent_path_test"), &files, None, true);

    assert!(!activity.is_git_repo);
    assert_eq!(activity.active_files, 0);
    assert_eq!(activity.frozen_files, 2);
    assert_eq!(activity.frozen_lines, 150);
    assert_eq!(activity.frozen_bytes, 4200);
    assert!(commit_map.is_empty());
}

#[test]
fn test_individual_token_estimators() {
    let disc = estimate_discovery_tokens(50, 2);
    assert!(disc > 0);

    let read = estimate_reading_tokens(50, 100);
    assert!(read > 0);

    let trace = estimate_tracing_tokens(3.5);
    assert_eq!(trace, 2 * 3 * 50);
}

#[test]
fn test_auto_window_days_zero_max_days_no_panic() {
    let days = auto_window_days(100, 0);
    assert_eq!(days, 7);
}

#[test]
fn test_scan_git_activity_subdirectory_matches_activity() {
    let temp = tempfile::TempDir::new().unwrap();
    let root = temp.path();

    let init = std::process::Command::new("git")
        .args(["init"])
        .current_dir(root)
        .output();
    if init.is_err() || !init.unwrap().status.success() {
        return;
    }
    let _ = std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(root)
        .output();
    let _ = std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(root)
        .output();

    let pkg_dir = root.join("pkg");
    std::fs::create_dir_all(&pkg_dir).unwrap();
    let file = pkg_dir.join("a.py");
    std::fs::write(&file, "x = 1\n").unwrap();

    let _ = std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(root)
        .output();
    let commit = std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(root)
        .output();
    if commit.is_err() || !commit.unwrap().status.success() {
        return;
    }

    let files = vec![ScannedFileInfo {
        path: file.clone(),
        lines: 1,
        bytes: 6,
    }];

    let (activity, commit_map) = scan_git_activity(&pkg_dir, &files, Some(1), false);
    assert_eq!(activity.active_files, 1);
    assert_eq!(activity.frozen_files, 0);
    assert_eq!(commit_map.get(&file).copied(), Some(1));

    // When the target itself is a file (e.g. context pkg/a.py)
    let (file_activity, file_commit_map) = scan_git_activity(&file, &files, Some(1), false);
    assert!(file_activity.is_git_repo);
    assert_eq!(file_activity.active_files, 1);
    assert_eq!(file_activity.frozen_files, 0);
    assert_eq!(file_commit_map.get(&file).copied(), Some(1));
}

#[test]
fn test_git_working_dir_normalizes_empty_parent() {
    use super::git_scanner::{git_working_dir, is_git_repository, resolve_git_root};

    assert_eq!(git_working_dir(Path::new("app.py")), Path::new("."));
    assert_eq!(git_working_dir(Path::new("")), Path::new("."));
    assert_eq!(git_working_dir(Path::new("src/app.py")), Path::new("src"));
    assert_eq!(git_working_dir(Path::new(".")), Path::new("."));

    // Bare filenames in repository root must correctly resolve Git repo and Git root
    assert!(is_git_repository(Path::new("Cargo.toml")));
    assert!(resolve_git_root(Path::new("Cargo.toml")).is_some());
}
