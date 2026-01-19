use regex::Regex;
use rustc_hash::FxHashSet;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Maximum recursion depth for AST visitor to prevent stack overflow on deeply nested code.
/// A depth of 400 is sufficient for any reasonable Python code while providing a safety margin.
pub const MAX_RECURSION_DEPTH: usize = 400;

/// Number of files to process per chunk in parallel processing.
/// Prevents OOM on very large projects by limiting concurrent memory usage.
pub const CHUNK_SIZE: usize = 500;

/// Default configuration filename.
pub const CONFIG_FILENAME: &str = ".cytoscnpy.toml";

/// Python project configuration filename.
pub const PYPROJECT_FILENAME: &str = "pyproject.toml";

/// Rule ID for configuration-related errors.
pub const RULE_ID_CONFIG_ERROR: &str = "CSP-CONFIG-ERROR";

/// Suppression comment patterns that disable findings on a line.
/// Supports both pragma format and noqa format.
/// - `# pragma: no cytoscnpy` - Legacy format
/// - `# noqa: CSP` - Standard Python linter format (with or without space after colon)
pub fn get_suppression_patterns() -> &'static [&'static str] {
    static PATTERNS: OnceLock<Vec<&'static str>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        vec![
            "pragma: no cytoscnpy", // Legacy format
            "noqa: CSP",            // New format (with space)
            "noqa:CSP",             // New format (no space)
        ]
    })
}

/// Regex for identifying suppression comments.
///
/// Supports:
/// - `# pragma: no cytoscnpy`
/// - `# noqa`, `# ignore`
/// - `# noqa: code1, code2` (and variants)
///
/// # Panics
///
/// Panics if the regex pattern is invalid.
pub fn get_suppression_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    #[allow(clippy::expect_used)]
    RE.get_or_init(|| {
        Regex::new(r"(?i)#\s*(?:pragma:\s*no\s*cytoscnpy|(?:noqa|ignore)(?::\s*([^#\n]+))?)")
            .expect("Invalid suppression regex pattern")
    })
}

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
        m.insert("module_constant", 80);
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
        Regex::new(r"(?i)(?:views|handlers|endpoints|routes|api|urls|function_app)\.py$")
            .expect("Invalid framework file regex pattern")
    })
}

/// Set of folders to exclude by default.
/// Includes Python, Node.js, Rust, Ruby, Java, and common IDE folders.
pub fn get_default_exclude_folders() -> &'static FxHashSet<&'static str> {
    static SET: OnceLock<FxHashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| {
        let mut s = FxHashSet::default();
        // Python
        s.insert("__pycache__");
        s.insert(".pytest_cache");
        s.insert(".mypy_cache");
        s.insert(".ruff_cache");
        s.insert(".tox");
        s.insert("htmlcov");
        s.insert(".coverage");
        s.insert("*.egg-info");
        s.insert(".eggs");
        s.insert("venv");
        s.insert(".venv");
        s.insert("env");
        s.insert(".env");
        s.insert(".nox");
        s.insert(".pytype");
        // Build outputs
        s.insert("build");
        s.insert("dist");
        s.insert("site-packages");
        // Node.js / JavaScript
        s.insert("node_modules");
        s.insert(".npm");
        s.insert("bower_components");
        // Rust
        s.insert("target");
        // Ruby
        s.insert("vendor");
        s.insert(".bundle");
        // Java / Gradle / Maven
        s.insert(".gradle");
        s.insert("gradle");
        s.insert(".mvn");
        // IDE and version control
        s.insert(".git");
        s.insert(".svn");
        s.insert(".hg");
        s.insert(".idea");
        s.insert(".vscode");
        s.insert(".vs");
        // Other
        s.insert(".cache");
        s.insert(".tmp");
        s.insert("tmp");
        s.insert("logs");
        s
    })
}

/// Set of pytest hooks that are automatically called.
#[allow(clippy::too_many_lines)]
pub fn get_pytest_hooks() -> &'static FxHashSet<&'static str> {
    static SET: OnceLock<FxHashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| {
        let mut s = FxHashSet::default();
        s.insert("main");
        s.insert("setup");
        s.insert("teardown");
        s.insert("pytest_configure");
        s.insert("pytest_sessionstart");
        s.insert("pytest_sessionfinish");
        s.insert("pytest_runtest_setup");
        s.insert("pytest_runtest_call");
        s.insert("pytest_runtest_teardown");
        s.insert("pytest_addoption");
        s.insert("pytest_collection_modifyitems");
        s.insert("pytest_generate_tests");
        s.insert("pytest_cmdline_preparse");
        s.insert("pytest_load_initial_conftests");
        s.insert("pytest_unconfigure");
        s.insert("pytest_exception_interact");
        s.insert("pytest_terminal_summary");
        s.insert("pytest_report_header");
        s.insert("pytest_collection_finish");
        s.insert("pytest_itemcollected");
        s.insert("pytest_deselected");
        s.insert("pytest_ignore_collect");
        s.insert("pytest_pycollect_makemodule");
        s.insert("pytest_pycollect_makeitem");
        s.insert("pytest_runtestloop");
        s.insert("pytest_runtest_protocol");
        s.insert("pytest_make_parametrize_id");
        s.insert("pytest_markeval");
        s.insert("pytest_namespace");
        s.insert("pytest_plugin_registered");
        s.insert("pytest_addhooks");
        s.insert("pytest_configure_node");
        s.insert("pytest_test_node_collect");
        s.insert("pytest_collection_node_collect");
        s.insert("pytest_collection_node_from_parent");
        s.insert("pytest_collection_node_from_path");
        s.insert("pytest_collection_node_from_module");
        s.insert("pytest_collection_node_from_function");
        s.insert("pytest_collection_node_from_class");
        s.insert("pytest_collection_node_from_file");
        s.insert("pytest_collection_node_from_dir");
        s.insert("pytest_collection_node_from_package");
        s.insert("pytest_collection_node_from_session");
        s.insert("pytest_collection_node_from_item");
        s.insert("pytest_collection_node_from_collector");
        s.insert("pytest_collection_node_from_fixture");
        s.insert("pytest_collection_node_from_hook");
        s.insert("pytest_collection_node_from_plugin");
        s.insert("pytest_collection_node_from_config");
        s.insert("pytest_collection_node_from_request");
        s.insert("pytest_collection_node_from_metafunc");
        s.insert("pytest_collection_node_from_call");
        s.insert("pytest_collection_node_from_result");
        s.insert("pytest_collection_node_from_report");
        s.insert("pytest_collection_node_from_error");
        s.insert("pytest_collection_node_from_warning");
        s.insert("pytest_collection_node_from_message");
        s.insert("pytest_collection_node_from_traceback");
        s.insert("pytest_collection_node_from_exception");
        s.insert("pytest_collection_node_from_outcome");
        s.insert("pytest_collection_node_from_duration");
        s.insert("pytest_collection_node_from_start");
        s.insert("pytest_collection_node_from_finish");
        s.insert("pytest_collection_node_from_setup");
        s.insert("pytest_collection_node_from_teardown");
        s.insert("pytest_collection_node_from_call_item");
        s.insert("pytest_collection_node_from_call_setup");
        s.insert("pytest_collection_node_from_call_teardown");
        s.insert("pytest_collection_node_from_call_fixture");
        s.insert("pytest_collection_node_from_call_hook");
        s.insert("pytest_collection_node_from_call_plugin");
        s.insert("pytest_collection_node_from_call_config");
        s.insert("pytest_collection_node_from_call_request");
        s.insert("pytest_collection_node_from_call_metafunc");
        s.insert("pytest_collection_node_from_call_result");
        s.insert("pytest_collection_node_from_call_report");
        s.insert("pytest_collection_node_from_call_error");
        s.insert("pytest_collection_node_from_call_warning");
        s.insert("pytest_collection_node_from_call_message");
        s.insert("pytest_collection_node_from_call_traceback");
        s.insert("pytest_collection_node_from_call_exception");
        s.insert("pytest_collection_node_from_call_outcome");
        s.insert("pytest_collection_node_from_call_duration");
        s.insert("pytest_collection_node_from_call_start");
        s.insert("pytest_collection_node_from_call_finish");
        s.insert("pytest_collection_node_from_call_setup_item");
        s.insert("pytest_collection_node_from_call_teardown_item");
        s.insert("pytest_collection_node_from_call_setup_fixture");
        s.insert("pytest_collection_node_from_call_teardown_fixture");
        s.insert("pytest_collection_node_from_call_setup_hook");
        s.insert("pytest_collection_node_from_call_teardown_hook");
        s.insert("pytest_collection_node_from_call_setup_plugin");
        s.insert("pytest_collection_node_from_call_teardown_plugin");
        s.insert("pytest_collection_node_from_call_setup_config");
        s.insert("pytest_collection_node_from_call_teardown_call_setup_config");
        s.insert("pytest_collection_node_from_call_setup_call_request");
        s.insert("pytest_collection_node_from_call_teardown_call_request");
        s.insert("pytest_collection_node_from_call_setup_call_metafunc");
        s.insert("pytest_collection_node_from_call_teardown_call_metafunc");
        s.insert("pytest_collection_node_from_call_setup_call_result");
        s.insert("pytest_collection_node_from_call_teardown_call_result");
        s.insert("pytest_collection_node_from_call_setup_call_report");
        s.insert("pytest_collection_node_from_call_teardown_call_report");
        s.insert("pytest_collection_node_from_call_setup_call_error");
        s.insert("pytest_collection_node_from_call_teardown_call_error");
        s.insert("pytest_collection_node_from_call_setup_call_warning");
        s.insert("pytest_collection_node_from_call_teardown_call_warning");
        s.insert("pytest_collection_node_from_call_setup_call_message");
        s.insert("pytest_collection_node_from_call_teardown_call_message");
        s.insert("pytest_collection_node_from_call_setup_call_traceback");
        s.insert("pytest_collection_node_from_call_teardown_call_traceback");
        s.insert("pytest_collection_node_from_call_setup_call_exception");
        s.insert("pytest_collection_node_from_call_teardown_call_exception");
        s.insert("pytest_collection_node_from_call_setup_call_outcome");
        s.insert("pytest_collection_node_from_call_teardown_call_outcome");
        s.insert("pytest_collection_node_from_call_setup_call_duration");
        s.insert("pytest_collection_node_from_call_teardown_call_duration");
        s.insert("pytest_collection_node_from_call_setup_call_start");
        s.insert("pytest_collection_node_from_call_teardown_call_start");
        s.insert("pytest_collection_node_from_call_setup_call_finish");
        s.insert("pytest_collection_node_from_call_teardown_call_finish");
        s
    })
}

/// Rules that are sensitive to taint analysis (injection, SSRF, path traversal).
pub fn get_taint_sensitive_rules() -> &'static [&'static str] {
    static RULES: OnceLock<Vec<&'static str>> = OnceLock::new();
    RULES.get_or_init(|| {
        vec![
            "CSP-D101", // SQL Injection (ORM)
            "CSP-D102", // SQL Injection (Raw)
            "CSP-D402", // SSRF
            "CSP-D501", // Path Traversal
        ]
    })
}

// Legacy aliases for backward compatibility
// Callers using PENALTIES.get("key") can use get_penalties().get("key") instead
pub use get_auto_called as AUTO_CALLED;
pub use get_default_exclude_folders as DEFAULT_EXCLUDE_FOLDERS;
pub use get_framework_file_re as FRAMEWORK_FILE_RE;
pub use get_penalties as PENALTIES;
pub use get_pytest_hooks as PYTEST_HOOKS;
pub use get_suppression_patterns as SUPPRESSION_PATTERNS;
pub use get_suppression_re as SUPPRESSION_RE;
pub use get_taint_sensitive_rules as TAINT_SENSITIVE_RULES;
pub use get_test_decor_re as TEST_DECOR_RE;
pub use get_test_file_re as TEST_FILE_RE;
pub use get_test_import_re as TEST_IMPORT_RE;
pub use get_test_method_pattern as TEST_METHOD_PATTERN;
pub use get_unittest_lifecycle_methods as UNITTEST_LIFECYCLE_METHODS;
