//! Tests for multi-target doctor health aggregation.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use cytoscnpy::doctor::{
    aggregate_doctor_results, ConfigCategory, DetectedConfig, DoctorResult, RepoStructureStats,
    SetupReliabilityScore, SetupVerdict,
};

#[test]
fn test_aggregate_doctor_results_empty() {
    assert!(aggregate_doctor_results(&[], Path::new(".")).is_none());
}

#[test]
fn test_aggregate_doctor_results_symmetry() {
    let make_doc = |name: &str, cat: ConfigCategory, has_lock: bool, has_ci: bool| DoctorResult {
        root_path: PathBuf::from(name),
        configs: vec![DetectedConfig {
            category: cat,
            name: format!("{name}_cfg"),
            path: PathBuf::from(format!("{name}/cfg")),
            details: None,
        }],
        pyproject: None,
        structure: RepoStructureStats {
            total_files: 5,
            total_lines: 100,
            total_bytes: 1000,
            source_files: 4,
            source_lines: 80,
            test_files: 1,
            test_lines: 20,
            test_to_source_ratio: 0.25,
            avg_file_lines: 20,
            largest_file_path: format!("{name}/app.py"),
            largest_file_lines: 50,
            max_directory_depth: 2,
            languages: vec![],
        },
        reliability: SetupReliabilityScore {
            score: 20,
            verdict: SetupVerdict::AtRisk,
            has_lockfile: has_lock,
            has_ci,
            has_tests: false,
            has_linter: false,
            has_formatter: false,
            has_type_checker: false,
            has_docker: false,
            has_readme_setup: false,
            has_build_script: false,
            has_gitignore: false,
            recommendations: vec![],
        },
    };

    let doc1 = make_doc("dir_a", ConfigCategory::Lockfile, true, false);
    let doc2 = make_doc("dir_b", ConfigCategory::CI, false, true);
    let root = Path::new("repo");

    let agg12 = aggregate_doctor_results(&[doc1.clone(), doc2.clone()], root).unwrap();
    let agg21 = aggregate_doctor_results(&[doc2, doc1], root).unwrap();

    assert_eq!(agg12, agg21);
    assert_eq!(agg12.structure.total_files, 10);
    assert_eq!(agg12.structure.total_lines, 200);
    assert!(agg12.reliability.has_lockfile);
    assert!(agg12.reliability.has_ci);
}
