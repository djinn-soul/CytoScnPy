mod limits;
mod penalties;
mod pytest_hooks;
mod regexes;
mod sets;
mod taint;

pub use limits::{
    BOILERPLATE_THRESHOLD, CHUNK_SIZE, CONFIG_FILENAME, MAX_RECURSION_DEPTH, MIN_CLONE_LINES,
    PYPROJECT_FILENAME, RULE_ID_CONFIG_ERROR, TAINT_ENABLED_DEFAULT,
};
pub use penalties::get_penalties;
pub use pytest_hooks::get_pytest_hooks;
pub use regexes::{
    get_fixture_decor_re, get_framework_file_re, get_suppression_patterns, get_suppression_re,
    get_test_decor_re, get_test_file_re, get_test_import_re, get_test_method_pattern,
};
pub use sets::{
    get_auto_called, get_default_exclude_folders, get_pytest_plugin_fixtures,
    get_unittest_lifecycle_methods,
};
pub use taint::get_taint_sensitive_rules;

pub use get_auto_called as AUTO_CALLED;
pub use get_default_exclude_folders as DEFAULT_EXCLUDE_FOLDERS;
pub use get_fixture_decor_re as FIXTURE_DECOR_RE;
pub use get_framework_file_re as FRAMEWORK_FILE_RE;
pub use get_penalties as PENALTIES;
pub use get_pytest_hooks as PYTEST_HOOKS;
pub use get_pytest_plugin_fixtures as PYTEST_PLUGIN_FIXTURES;
pub use get_suppression_patterns as SUPPRESSION_PATTERNS;
pub use get_suppression_re as SUPPRESSION_RE;
pub use get_taint_sensitive_rules as TAINT_SENSITIVE_RULES;
pub use get_test_decor_re as TEST_DECOR_RE;
pub use get_test_file_re as TEST_FILE_RE;
pub use get_test_import_re as TEST_IMPORT_RE;
pub use get_test_method_pattern as TEST_METHOD_PATTERN;
pub use get_unittest_lifecycle_methods as UNITTEST_LIFECYCLE_METHODS;
