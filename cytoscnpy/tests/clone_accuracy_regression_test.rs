//! Curated precision/recall regressions for exact normalized clone matching.

use cytoscnpy::clones::{CloneConfig, CloneDetector};
use std::path::PathBuf;

fn detects_pair(left: &str, right: &str) -> bool {
    detects_pair_at_similarity(left, right, 1.0)
}

fn detects_pair_at_similarity(left: &str, right: &str, min_similarity: f64) -> bool {
    let files = vec![
        (PathBuf::from("src/left.py"), left.to_owned()),
        (PathBuf::from("src/right.py"), right.to_owned()),
    ];
    let config = CloneConfig::default()
        .with_min_similarity(min_similarity)
        .with_tests(true);
    let Ok(detector) = CloneDetector::with_config(config) else {
        return false;
    };
    !detector.detect(&files).pairs.is_empty()
}

/// True when the two named definitions are reported as a clone pair at
/// exact (normalized) similarity.
fn detects_named_pair(left: &str, right: &str, left_name: &str, right_name: &str) -> bool {
    let files = vec![
        (PathBuf::from("src/left.py"), left.to_owned()),
        (PathBuf::from("src/right.py"), right.to_owned()),
    ];
    let config = CloneConfig::default()
        .with_min_similarity(1.0)
        .with_tests(true);
    let Ok(detector) = CloneDetector::with_config(config) else {
        return false;
    };
    detector.detect(&files).pairs.iter().any(|pair| {
        let names = (
            pair.instance_a.name.as_deref(),
            pair.instance_b.name.as_deref(),
        );
        names == (Some(left_name), Some(right_name)) || names == (Some(right_name), Some(left_name))
    })
}

#[path = "clone_accuracy_regression_test/cases_1.rs"]
mod cases_1;
#[path = "clone_accuracy_regression_test/cases_2.rs"]
mod cases_2;
#[path = "clone_accuracy_regression_test/cases_3.rs"]
mod cases_3;
