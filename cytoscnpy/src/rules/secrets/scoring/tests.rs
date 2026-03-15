//! Tests for scoring engine.

#[cfg(test)]
use crate::rules::secrets::scoring::{ContextScorer, ScoringContext};
#[cfg(test)]
use std::path::{Path, PathBuf};

#[test]
fn test_scorer_test_file_detection() {
    assert!(ContextScorer::is_test_file(Path::new(
        "/project/tests/test_secrets.py"
    )));
    assert!(ContextScorer::is_test_file(Path::new(
        "/project/test/test_main.py"
    )));
    assert!(ContextScorer::is_test_file(Path::new(
        "/project/src/test_utils.py"
    )));
    assert!(ContextScorer::is_test_file(Path::new(
        "/project/conftest.py"
    )));
    assert!(!ContextScorer::is_test_file(Path::new(
        "/project/src/main.py"
    )));
}

#[test]
fn test_scorer_env_var_detection() {
    assert!(ContextScorer::is_env_var_access(
        "password = os.environ.get('PASSWORD')"
    ));
    assert!(ContextScorer::is_env_var_access(
        "key = os.getenv('API_KEY')"
    ));
    assert!(!ContextScorer::is_env_var_access("password = 'hardcoded'"));
}

#[test]
fn test_scorer_placeholder_detection() {
    assert!(ContextScorer::is_placeholder("api_key = 'xxx123'"));
    assert!(ContextScorer::is_placeholder("secret = 'your_secret_here'"));
    assert!(ContextScorer::is_placeholder("token = '${TOKEN}'"));
    assert!(!ContextScorer::is_placeholder("api_key = 'sk_live_abc123'"));
}

#[test]
fn test_scorer_scoring() {
    let scorer = ContextScorer::new();
    let path = PathBuf::from("/project/src/main.py");

    let ctx = ScoringContext {
        line_content: "password = 'secret123'",
        file_path: &path,
        is_comment: false,
        is_docstring: false,
        is_test_file: false,
    };

    // Base score should remain unchanged for normal context
    assert_eq!(scorer.score(70, &ctx), 70);

    // Test file should reduce score
    let test_path = PathBuf::from("/project/tests/test_main.py");
    let test_ctx = ScoringContext {
        line_content: "password = 'secret123'",
        file_path: &test_path,
        is_comment: false,
        is_docstring: false,
        is_test_file: true,
    };
    assert_eq!(scorer.score(70, &test_ctx), 20); // 70 - 50 = 20

    // Env var should reduce score to 0
    let env_ctx = ScoringContext {
        line_content: "password = os.environ.get('PASSWORD')",
        file_path: &path,
        is_comment: false,
        is_docstring: false,
        is_test_file: false,
    };
    assert_eq!(scorer.score(70, &env_ctx), 0); // 70 - 100, clamped to 0
}
