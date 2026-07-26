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
    let Ok(detector) = CloneDetector::with_config(config) else {
        return false;
    };
    !detector.detect(&files).pairs.is_empty()
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

#[test]
fn enum_member_names_and_values_remain_semantic() {
    let payment_states = "from enum import Enum\n\nclass PaymentState(Enum):\n    PENDING = \"pending\"\n    SETTLED = \"settled\"\n    FAILED = \"failed\"\n    REFUNDED = \"refunded\"\n";
    let access_levels = "from enum import Enum\n\nclass AccessLevel(Enum):\n    GUEST = \"guest\"\n    MEMBER = \"member\"\n    ADMIN = \"admin\"\n    OWNER = \"owner\"\n";
    let renamed_payment_states = "from enum import Enum\n\nclass TransactionState(Enum):\n    PENDING = 'pending'\n    SETTLED = 'settled'\n    FAILED = 'failed'\n    REFUNDED = 'refunded'\n";

    assert!(
        !detects_pair(payment_states, access_levels),
        "different enum members must not be reported as clones"
    );
    assert!(
        detects_pair(payment_states, renamed_payment_states),
        "renaming only the enum class should remain a Type-2 clone"
    );
}

#[test]
fn qualified_enum_bases_preserve_member_semantics() {
    let colors = "import enum\n\nclass Color(enum.IntEnum):\n    RED = 1\n    GREEN = 2\n    BLUE = 3\n    WHITE = 4\n";
    let priorities = "import enum\n\nclass Priority(enum.IntEnum):\n    LOW = 10\n    MEDIUM = 20\n    HIGH = 30\n    URGENT = 40\n";

    assert!(
        !detects_pair(colors, priorities),
        "qualified enum bases must preserve member semantics"
    );
}
