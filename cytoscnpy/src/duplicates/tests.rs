use super::analyzer::{analyze_duplicates_files, DuplicatesOptions};
use super::interval::{count_non_overlapping_lines, merge_intervals};
use super::reporter::{print_json_report, print_terminal_report};
use super::types::{DuplicateCluster, DuplicateLocation, DuplicatesResult};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_merge_intervals_empty() {
    let intervals: Vec<(usize, usize)> = Vec::new();
    assert_eq!(merge_intervals(intervals), Vec::new());
}

#[test]
fn test_merge_intervals_disjoint() {
    let intervals = vec![(1, 5), (10, 15), (20, 25)];
    assert_eq!(merge_intervals(intervals), vec![(1, 5), (10, 15), (20, 25)]);
}

#[test]
fn test_merge_intervals_overlapping() {
    let intervals = vec![(1, 10), (5, 15), (12, 20)];
    assert_eq!(merge_intervals(intervals), vec![(1, 20)]);
}

#[test]
fn test_merge_intervals_adjacent() {
    let intervals = vec![(1, 10), (11, 20)];
    assert_eq!(merge_intervals(intervals), vec![(1, 20)]);
}

#[test]
fn test_merge_intervals_contained_and_unordered() {
    let intervals = vec![(10, 15), (1, 30), (5, 12)];
    assert_eq!(merge_intervals(intervals), vec![(1, 30)]);
}

#[test]
fn test_count_non_overlapping_lines() {
    // Two intervals overlapping: 1..10 (10 lines) and 5..15 (11 lines)
    // Naive sum would be 21, but actual union 1..15 is 15 lines.
    let intervals = vec![(1, 10), (5, 15)];
    assert_eq!(count_non_overlapping_lines(intervals), 15);

    // Disjoint intervals: 1..5 (5 lines) and 10..15 (6 lines) -> 11 lines
    let disjoint = vec![(1, 5), (10, 15)];
    assert_eq!(count_non_overlapping_lines(disjoint), 11);
}

#[test]
fn test_location_line_span() {
    let loc = DuplicateLocation {
        file: "test.py".into(),
        start_line: 10,
        end_line: 25,
        name: Some("process_data".to_owned()),
        node_kind: "function".to_owned(),
    };
    assert_eq!(loc.line_span(), 16);
}

#[test]
fn test_result_is_clean() {
    let mut res = DuplicatesResult::default();
    assert!(res.is_clean());

    res.clusters.push(DuplicateCluster {
        id: 1,
        clone_type: "Exact".to_owned(),
        similarity: 1.0,
        line_count: 10,
        locations: Vec::new(),
    });
    assert!(!res.is_clean());
}

#[test]
fn test_analyze_duplicates_clean_file() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("clean.py");
    fs::write(
        &file,
        r"def unique_func_one(x):
    return x * 42

def unique_func_two(y):
    import math
    return math.sqrt(y) + 7
",
    )
    .unwrap();

    let options = DuplicatesOptions::default();
    let res = analyze_duplicates_files(&[file], &options);
    assert_eq!(res.clusters.len(), 0);
    assert_eq!(res.stats.cluster_count, 0);
    assert_eq!(res.stats.total_duplicate_lines, 0);
    assert!(res.is_clean());
}

#[test]
fn test_analyze_duplicates_with_clones() {
    let dir = tempdir().unwrap();
    let file1 = dir.path().join("a.py");
    let file2 = dir.path().join("b.py");

    let duplicate_code = r"def calculate_metrics(items):
    total = 0
    count = 0
    for item in items:
        if item > 0:
            total += item
            count += 1
    if count == 0:
        return 0.0
    return total / count
";

    fs::write(&file1, duplicate_code).unwrap();
    fs::write(&file2, duplicate_code).unwrap();

    let options = DuplicatesOptions {
        min_lines: 4,
        min_similarity: 0.80,
        ..Default::default()
    };

    let res = analyze_duplicates_files(&[file1, file2], &options);
    assert!(!res.clusters.is_empty());
    assert!(res.stats.total_duplicate_lines > 0);
    assert!(res.stats.duplicate_pct > 0.0);
    assert_eq!(res.stats.affected_files, 2);

    // Verify terminal and JSON reports don't panic
    let mut term_out = Vec::new();
    print_terminal_report(&res, Some(dir.path()), &mut term_out).unwrap();
    let term_str = String::from_utf8(term_out).unwrap();
    assert!(term_str.contains("Duplicate Code Clusters"));
    assert!(term_str.contains("Duplicate clusters"));

    let mut json_out = Vec::new();
    print_json_report(&res, &mut json_out).unwrap();
    let json_str = String::from_utf8(json_out).unwrap();
    assert!(json_str.contains("\"cluster_count\""));
}
