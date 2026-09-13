use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use super::types::{GitActivity, HotFile};

/// File metadata collected during repository scanning.
#[derive(Debug, Clone)]
pub struct ScannedFileInfo {
    /// File path.
    pub path: PathBuf,
    /// Number of lines in the file.
    pub lines: usize,
    /// Size of the file in bytes.
    pub bytes: u64,
}

/// Checks whether a given directory is part of a Git repository.
pub fn is_git_repository(repo_path: &Path) -> bool {
    if repo_path.join(".git").exists() {
        return true;
    }
    let output = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(repo_path)
        .output();
    matches!(output, Ok(o) if o.status.success())
}

/// Computes the age of the repository in days from its initial commit.
pub fn repo_age_days(repo_path: &Path) -> Option<u32> {
    let root = Command::new("git")
        .args(["rev-list", "--max-parents=0", "HEAD"])
        .current_dir(repo_path)
        .output()
        .ok()?;

    if !root.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&root.stdout);
    let root_hash = stdout.lines().next()?.trim();
    if root_hash.is_empty() {
        return None;
    }

    let ts_output = Command::new("git")
        .args(["log", "-1", "--format=%at", root_hash])
        .current_dir(repo_path)
        .output()
        .ok()?;

    if !ts_output.status.success() {
        return None;
    }

    let ts_str = String::from_utf8_lossy(&ts_output.stdout);
    let timestamp: u64 = ts_str.trim().parse().ok()?;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()?
        .as_secs();

    let age_secs = now.saturating_sub(timestamp);
    Some(u32::try_from(age_secs / 86400).unwrap_or(u32::MAX))
}

/// Automatically determines window days, clamped between 7 and `max_days`.
#[must_use]
pub fn auto_window_days(repo_age: u32, max_days: u32) -> u32 {
    (repo_age / 3).clamp(7, max_days)
}

/// Formats window days into human-friendly duration text.
#[must_use]
pub fn format_window_label(days: u32) -> String {
    if days < 14 {
        format!("{days} days")
    } else if days < 60 {
        format!("{} weeks", days / 7)
    } else {
        format!("{} months", days / 30)
    }
}

/// Parses raw `git log --name-only` output into file-level commit counts.
pub fn parse_file_frequency(git_log_output: &str) -> HashMap<PathBuf, usize> {
    let mut counts: HashMap<PathBuf, usize> = HashMap::new();
    for line in git_log_output.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            *counts.entry(PathBuf::from(trimmed)).or_insert(0) += 1;
        }
    }
    counts
}

/// Fetches commit frequency for all modified files within `days` lookback.
pub fn git_file_frequency_days(repo_path: &Path, days: u32) -> Option<HashMap<PathBuf, usize>> {
    let since = format!("{days} days ago");
    let output = Command::new("git")
        .args([
            "log",
            "--since",
            &since,
            "--format=",
            "--name-only",
            "--diff-filter=ACMR",
        ])
        .current_dir(repo_path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(parse_file_frequency(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

/// Counts total commits created in the repository within `days` lookback.
pub fn count_commits_days(repo_path: &Path, days: u32) -> Option<usize> {
    let since = format!("{days} days ago");
    let output = Command::new("git")
        .args(["rev-list", "--count", "--since", &since, "HEAD"])
        .current_dir(repo_path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

/// Makes a target file path relative to repo root.
pub fn relativize_path(repo_path: &Path, file_path: &Path) -> PathBuf {
    if let (Ok(repo_can), Ok(file_can)) = (repo_path.canonicalize(), file_path.canonicalize()) {
        if let Ok(rel) = file_can.strip_prefix(&repo_can) {
            return rel.to_path_buf();
        }
    }
    if let Ok(rel) = file_path.strip_prefix(repo_path) {
        return rel.to_path_buf();
    }
    file_path.to_path_buf()
}

/// Reads lines and byte counts for a file.
pub fn collect_file_info(file_path: &Path) -> Option<ScannedFileInfo> {
    let meta = fs::metadata(file_path).ok()?;
    let content = fs::read_to_string(file_path).ok()?;
    let lines = content.lines().count();
    Some(ScannedFileInfo {
        path: file_path.to_path_buf(),
        lines,
        bytes: meta.len(),
    })
}

/// Scans Git repository activity across scanned files.
pub fn scan_git_activity(
    repo_path: &Path,
    files: &[ScannedFileInfo],
    months_override: Option<u32>,
    no_git: bool,
) -> (GitActivity, HashMap<PathBuf, usize>) {
    if no_git || !is_git_repository(repo_path) {
        let (f_lines, f_bytes) = files
            .iter()
            .fold((0, 0u64), |(l, b), f| (l + f.lines, b + f.bytes));
        return (
            GitActivity {
                is_git_repo: false,
                frozen_files: files.len(),
                frozen_lines: f_lines,
                frozen_bytes: f_bytes,
                ..GitActivity::default()
            },
            HashMap::new(),
        );
    }

    let max_days = months_override.unwrap_or(1) * 30;
    let window_days = repo_age_days(repo_path)
        .map(|age| auto_window_days(age, max_days))
        .unwrap_or(max_days);

    let file_commits = git_file_frequency_days(repo_path, window_days).unwrap_or_default();
    let total_commits = count_commits_days(repo_path, window_days).unwrap_or(0);

    let mut active_files = 0usize;
    let mut active_lines = 0usize;
    let mut active_bytes = 0u64;
    let mut frozen_files = 0usize;
    let mut frozen_lines = 0usize;
    let mut frozen_bytes = 0u64;
    let mut hot_files: Vec<HotFile> = Vec::new();
    let mut commit_map_by_path: HashMap<PathBuf, usize> = HashMap::new();

    for file in files {
        let rel_path = relativize_path(repo_path, &file.path);
        let commit_count = file_commits
            .get(&rel_path)
            .copied()
            .or_else(|| file_commits.get(&file.path).copied())
            .unwrap_or(0);

        if commit_count > 0 {
            active_files += 1;
            active_lines += file.lines;
            active_bytes += file.bytes;
            commit_map_by_path.insert(file.path.clone(), commit_count);
            hot_files.push(HotFile {
                path: file.path.clone(),
                display_path: rel_path.display().to_string(),
                commit_count,
                lines: file.lines,
                bytes: file.bytes,
            });
        } else {
            frozen_files += 1;
            frozen_lines += file.lines;
            frozen_bytes += file.bytes;
        }
    }

    hot_files.sort_by_key(|a| std::cmp::Reverse(a.commit_count));
    hot_files.truncate(10);

    (
        GitActivity {
            is_git_repo: true,
            active_files,
            active_lines,
            active_bytes,
            frozen_files,
            frozen_lines,
            frozen_bytes,
            total_commits,
            window_days,
            window_label: format_window_label(window_days),
            hot_files,
        },
        commit_map_by_path,
    )
}
