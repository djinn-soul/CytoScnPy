//! Curated precision/recall regressions for exact normalized clone matching.

use cytoscnpy::clones::{CloneConfig, CloneDetector};
use std::path::PathBuf;

fn detects_pair(left: &str, right: &str) -> bool {
    let files = vec![
        (PathBuf::from("src/left.py"), left.to_owned()),
        (PathBuf::from("src/right.py"), right.to_owned()),
    ];
    let config = CloneConfig::default()
        .with_min_similarity(1.0)
        .with_tests(true);
    !CloneDetector::with_config(config)
        .detect(&files)
        .pairs
        .is_empty()
}

#[test]
fn curated_exact_clone_corpus_has_full_precision_and_recall() {
    let positives = [
        (
            "def render(values):\n    selected = [value * 2 for value in values if value > 0]\n    total = sum(selected)\n    message = f\"total={total}\"\n    return message\n",
            "def display(items):\n    chosen = [item * 2 for item in items if item > 0]\n    amount = sum(chosen)\n    text = f\"total={amount}\"\n    return text\n",
        ),
        (
            "def classify(value):\n    match value:\n        case [first, *rest] if first:\n            return first\n        case _:\n            return None\n",
            "def categorize(item):\n    match item:\n        case [head, *tail] if head:\n            return head\n        case _:\n            return None\n",
        ),
    ];
    let negatives = [
        (
            "def calculate(value):\n    first = value + 1\n    second = first + 2\n    third = second + 3\n    return third\n",
            "def calculate(value):\n    first = value - 1\n    second = first - 2\n    third = second - 3\n    return third\n",
        ),
        (
            "@cache\ndef load(value: int):\n    first = prepare(value, mode=True)\n    second = finalize(first)\n    return second\n",
            "@property\ndef load(value: str):\n    first = prepare(value, policy=True)\n    second = finalize(first)\n    return second\n",
        ),
        (
            "def unpack(value):\n    match value:\n        case [first, *rest]:\n            return first\n        case _:\n            return None\n",
            "def unpack(value):\n    match value:\n        case {\"first\": first, **rest}:\n            return first\n        case _:\n            return None\n",
        ),
    ];

    let true_positives = positives
        .iter()
        .filter(|(left, right)| detects_pair(left, right))
        .count();
    let false_positives = negatives
        .iter()
        .filter(|(left, right)| detects_pair(left, right))
        .count();

    assert_eq!(true_positives, positives.len(), "curated recall regressed");
    assert_eq!(false_positives, 0, "curated precision regressed");
}
