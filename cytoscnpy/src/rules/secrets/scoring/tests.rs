//! Tests for scoring engine.

#[cfg(test)]
use crate::rules::secrets::scoring::{ContextScorer, ScoringContext};
#[cfg(test)]
use std::path::{Path, PathBuf};

#[test]
fn test_scorer_test_file_detection() {
    let scorer = ContextScorer::new();

    assert!(scorer.is_test_file(Path::new("/project/tests/test_secrets.py")));
    assert!(scorer.is_test_file(Path::new("/project/test/test_main.py")));
    assert!(scorer.is_test_file(Path::new("/project/src/test_utils.py")));
    assert!(scorer.is_test_file(Path::new("/project/conftest.py")));
    assert!(!scorer.is_test_file(Path::new("/project/src/main.py")));
}

#[test]
fn test_scorer_env_var_detection() {
    let scorer = ContextScorer::new();

    assert!(scorer.is_env_var_access("password = os.environ.get('PASSWORD')"));
    assert!(scorer.is_env_var_access("key = os.getenv('API_KEY')"));
    assert!(!scorer.is_env_var_access("password = 'hardcoded'"));
}

#[test]
fn test_scorer_placeholder_detection() {
    let scorer = ContextScorer::new();

    assert!(scorer.is_placeholder("api_key = 'xxx123'"));
    assert!(scorer.is_placeholder("secret = 'your_secret_here'"));
    assert!(scorer.is_placeholder("token = '${TOKEN}'"));
    assert!(!scorer.is_placeholder("api_key = 'sk_live_abc123'"));
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
    };
    assert_eq!(scorer.score(70, &test_ctx), 20); // 70 - 50 = 20

    // Env var should reduce score to 0
    let env_ctx = ScoringContext {
        line_content: "password = os.environ.get('PASSWORD')",
        file_path: &path,
        is_comment: false,
        is_docstring: false,
    };
    assert_eq!(scorer.score(70, &env_ctx), 0); // 70 - 100, clamped to 0
}
