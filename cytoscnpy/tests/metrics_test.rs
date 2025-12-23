//! Tests for Maintainability Index (MI) metrics.
#![allow(clippy::float_cmp)]

use cytoscnpy::metrics::{mi_compute, mi_rank};

#[test]
fn test_mi_compute_simple() {
    // Example values
    let volume = 100.0;
    let complexity = 5;
    let sloc = 20;
    let comments = 0;

    // MI = 171 - 5.2 * ln(100) - 0.23 * 5 - 16.2 * ln(20)
    // MI = 171 - 5.2 * 4.605 - 1.15 - 16.2 * 2.995
    // MI = 171 - 23.946 - 1.15 - 48.519
    // MI = 97.385

    let score = mi_compute(volume, complexity, sloc, comments);
    assert!(score > 97.0 && score < 98.0);
    assert_eq!(mi_rank(score), 'A');
}

#[test]
fn test_mi_compute_with_comments() {
    let volume = 100.0;
    let complexity = 5;
    let sloc = 20;
    let comments = 5;

    // Base MI = 97.385
    // Comment weight = 50 * sin(sqrt(2.4 * (5/20)))
    // = 50 * sin(sqrt(2.4 * 0.25))
    // = 50 * sin(sqrt(0.6))
    // = 50 * sin(0.7746)
    // = 50 * 0.699
    // = 34.95

    // Total MI = 97.385 + 34.95 = 132.335 -> clamped to 100

    let score = mi_compute(volume, complexity, sloc, comments);
    assert_eq!(score, 100.0);
    assert_eq!(mi_rank(score), 'A');
}

#[test]
fn test_mi_rank() {
    assert_eq!(mi_rank(100.0), 'A');
    assert_eq!(mi_rank(20.0), 'A');
    assert_eq!(mi_rank(19.9), 'B');
    assert_eq!(mi_rank(10.0), 'B');
    assert_eq!(mi_rank(9.9), 'C');
    assert_eq!(mi_rank(0.0), 'C');
}
