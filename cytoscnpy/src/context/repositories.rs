//! Aggregate repository-local Git history without mixing identical relative paths.
use super::git_scanner::{resolve_git_root, scan_git_activity};
use super::hotspots::find_hotspots_with_sources;
use super::{ContextConfig, GitActivity, HotspotFile, ScannedFileInfo};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub(super) fn analyze_repositories(
    files: &[ScannedFileInfo],
    config: &ContextConfig,
    sources: Option<&crate::utils::sources::SourceCache>,
) -> (GitActivity, Vec<HotspotFile>) {
    let mut roots = BTreeMap::new();
    let mut groups: BTreeMap<PathBuf, Vec<ScannedFileInfo>> = BTreeMap::new();
    for file in files {
        let parent = file.path.parent().unwrap_or(std::path::Path::new("."));
        let root = roots.entry(parent.to_path_buf()).or_insert_with(|| {
            if config.no_git {
                None
            } else {
                resolve_git_root(parent)
            }
        });
        let key = root.clone().unwrap_or_else(|| parent.to_path_buf());
        groups.entry(key).or_default().push(file.clone());
    }
    let multiple = groups.len() > 1;
    let mut combined = GitActivity::default();
    if !groups.is_empty() {
        combined.window_days = 0;
    }
    let mut hotspots = Vec::new();
    for (root, files) in groups {
        let (mut activity, commits) =
            scan_git_activity(&root, &files, config.git_months, config.no_git);
        let paths: Vec<_> = files.iter().map(|file| file.path.clone()).collect();
        let mut found = find_hotspots_with_sources(&root, &paths, &commits, sources);
        if multiple {
            for path in &mut activity.frozen_paths {
                *path = root.join(&*path);
            }
            for hot in &mut activity.hot_files {
                hot.display_path = hot.path.display().to_string();
            }
            for hot in &mut found {
                hot.display_path = hot.path.display().to_string();
            }
        }
        combined.is_git_repo |= activity.is_git_repo;
        combined.active_files += activity.active_files;
        combined.active_lines += activity.active_lines;
        combined.active_bytes += activity.active_bytes;
        combined.frozen_files += activity.frozen_files;
        combined.frozen_lines += activity.frozen_lines;
        combined.frozen_bytes += activity.frozen_bytes;
        combined.total_commits += activity.total_commits;
        combined.window_days = combined.window_days.max(activity.window_days);
        combined.frozen_paths.extend(activity.frozen_paths);
        combined.hot_files.extend(activity.hot_files);
        hotspots.extend(found);
    }
    combined.window_label = super::git_scanner::format_window_label(combined.window_days);
    combined.frozen_paths.sort();
    combined.hot_files.sort_by(|a, b| {
        b.commit_count
            .cmp(&a.commit_count)
            .then_with(|| a.path.cmp(&b.path))
    });
    combined.hot_files.truncate(10);
    hotspots.sort_by(|a, b| {
        b.risk_score
            .cmp(&a.risk_score)
            .then_with(|| a.path.cmp(&b.path))
    });
    (combined, hotspots)
}
