use regex::Regex;
use std::sync::OnceLock;

/// Returns supported inline suppression token patterns.
pub fn get_suppression_patterns() -> &'static [&'static str] {
    static PATTERNS: OnceLock<Vec<&'static str>> = OnceLock::new();
    PATTERNS.get_or_init(|| vec!["pragma: no cytoscnpy", "noqa: CSP", "noqa:CSP"])
}

/// Returns the compiled suppression-comment regex.
pub fn get_suppression_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    #[allow(clippy::expect_used)]
    RE.get_or_init(|| {
        Regex::new(r"(?i)#\s*(?:pragma:\s*no\s*cytoscnpy|(?:noqa|ignore)(?::\s*([^#\n]+))?)")
            .expect("Invalid suppression regex pattern")
    })
}

/// Returns the compiled regex for test-file path detection.
pub fn get_test_file_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    #[allow(clippy::expect_used)]
    RE.get_or_init(|| {
        Regex::new(
            r"(?:^|[/\\])tests?[/\\]|(?:^|[/\\])test_[^/\\]+\.py$|[^/\\]+_test\.py$|conftest\.py$",
        )
        .expect("Invalid test file regex pattern")
    })
}

/// Returns the compiled regex for test-framework imports.
pub fn get_test_import_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    #[allow(clippy::expect_used)]
    RE.get_or_init(|| {
        Regex::new(r"^(pytest|unittest|nose|mock|responses)(\.|$)")
            .expect("Invalid test import regex pattern")
    })
}

/// Returns the compiled regex for fixture decorators.
pub fn get_fixture_decor_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    #[allow(clippy::expect_used)]
    RE.get_or_init(|| {
        Regex::new(
            r"(?x)^(
            pytest\.fixture |
            fixture
        )$",
        )
        .expect("Invalid fixture decorator regex pattern")
    })
}

/// Returns the compiled regex for test-specific decorators.
pub fn get_test_decor_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    #[allow(clippy::expect_used)]
    RE.get_or_init(|| {
        Regex::new(
            r"(?x)^(
            pytest\.(fixture|mark) |
            fixture |
            patch(\.|$) |
            responses\.activate |
            freeze_time
        )$",
        )
        .expect("Invalid test decorator regex pattern")
    })
}

/// Returns the compiled regex for `test_*` method names.
pub fn get_test_method_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    #[allow(clippy::expect_used)]
    RE.get_or_init(|| Regex::new(r"^test_\w+$").expect("Invalid test method regex pattern"))
}

/// Returns the compiled regex for framework-convention file names.
pub fn get_framework_file_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    #[allow(clippy::expect_used)]
    RE.get_or_init(|| {
        Regex::new(r"(?i)(?:views|handlers|endpoints|routes|api|urls|function_app)\.py$")
            .expect("Invalid framework file regex pattern")
    })
}
