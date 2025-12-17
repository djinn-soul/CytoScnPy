use regex::Regex;
use rustc_hash::FxHashSet;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Confidence adjustment penalties for various code patterns.
pub fn get_penalties() -> &'static HashMap<&'static str, u8> {
    static PENALTIES: OnceLock<HashMap<&'static str, u8>> = OnceLock::new();
    PENALTIES.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert("private_name", 80);
        m.insert("dunder_or_magic", 100);
        m.insert("underscored_var", 100);
        m.insert("in_init_file", 15);
        m.insert("dynamic_module", 40);
        m.insert("test_related", 100);
        m.insert("framework_magic", 40);
        m.insert("type_checking_import", 100); // TYPE_CHECKING imports are type-only
        m
    })
}

/// Regex for identifying test files.
///
/// # Panics
///
/// Panics if the regex pattern is invalid.
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

/// Regex for identifying imports of testing frameworks.
///
/// # Panics
///
/// Panics if the regex pattern is invalid.
pub fn get_test_import_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    #[allow(clippy::expect_used)]
    RE.get_or_init(|| {
        Regex::new(r"^(pytest|unittest|nose|mock|responses)(\.|$)")
            .expect("Invalid test import regex pattern")
    })
}

/// Regex for identifying test-related decorators.
///
/// # Panics
///
/// Panics if the regex pattern is invalid.
pub fn get_test_decor_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    #[allow(clippy::expect_used)]
    RE.get_or_init(|| {
        Regex::new(
            r"(?x)^(
            pytest\.(fixture|mark) |
            patch(\.|$) |
            responses\.activate |
            freeze_time
        )$",
        )
        .expect("Invalid test decorator regex pattern")
    })
}

/// Set of method names that are automatically called by Python (magic methods).
pub fn get_auto_called() -> &'static FxHashSet<&'static str> {
    static SET: OnceLock<FxHashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| {
        let mut s = FxHashSet::default();
        s.insert("__init__");
        s.insert("__enter__");
        s.insert("__exit__");
        s
    })
}

/// Regex for identifying test methods.
///
/// # Panics
///
/// Panics if the regex pattern is invalid.
pub fn get_test_method_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    #[allow(clippy::expect_used)]
    RE.get_or_init(|| Regex::new(r"^test_\w+$").expect("Invalid test method regex pattern"))
}

/// Set of unittest lifecycle methods.
pub fn get_unittest_lifecycle_methods() -> &'static FxHashSet<&'static str> {
    static SET: OnceLock<FxHashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| {
        let mut s = FxHashSet::default();
        s.insert("setUp");
        s.insert("tearDown");
        s.insert("setUpClass");
        s.insert("tearDownClass");
        s.insert("setUpModule");
        s.insert("tearDownModule");
        s
    })
}

/// Regex for identifying framework-specific files (e.g. Django, Flask patterns).
///
/// # Panics
///
/// Panics if the regex pattern is invalid.
pub fn get_framework_file_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    #[allow(clippy::expect_used)]
    RE.get_or_init(|| {
        Regex::new(r"(?i)(?:views|handlers|endpoints|routes|api|urls)\.py$")
            .expect("Invalid framework file regex pattern")
    })
}

/// Set of folders to exclude by default.
pub fn get_default_exclude_folders() -> &'static FxHashSet<&'static str> {
    static SET: OnceLock<FxHashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| {
        let mut s = FxHashSet::default();
        s.insert("__pycache__");
        s.insert(".git");
        s.insert(".pytest_cache");
        s.insert(".mypy_cache");
        s.insert(".tox");
        s.insert("htmlcov");
        s.insert(".coverage");
        s.insert("build");
        s.insert("dist");
        s.insert("*.egg-info");
        s.insert("venv");
        s.insert(".venv");
        s
    })
}

// Legacy aliases for backward compatibility
// Callers using PENALTIES.get("key") can use get_penalties().get("key") instead
pub use get_auto_called as AUTO_CALLED;
pub use get_default_exclude_folders as DEFAULT_EXCLUDE_FOLDERS;
pub use get_framework_file_re as FRAMEWORK_FILE_RE;
pub use get_penalties as PENALTIES;
pub use get_test_decor_re as TEST_DECOR_RE;
pub use get_test_file_re as TEST_FILE_RE;
pub use get_test_import_re as TEST_IMPORT_RE;
pub use get_test_method_pattern as TEST_METHOD_PATTERN;
pub use get_unittest_lifecycle_methods as UNITTEST_LIFECYCLE_METHODS;
