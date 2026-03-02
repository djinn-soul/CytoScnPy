use rustc_hash::FxHashSet;
use std::sync::OnceLock;

/// Returns the set of auto-invoked Python magic method names.
pub fn get_auto_called() -> &'static FxHashSet<&'static str> {
    static SET: OnceLock<FxHashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| {
        let mut set = FxHashSet::default();
        set.insert("__init__");
        set.insert("__enter__");
        set.insert("__exit__");
        set
    })
}

/// Returns unittest lifecycle method names.
pub fn get_unittest_lifecycle_methods() -> &'static FxHashSet<&'static str> {
    static SET: OnceLock<FxHashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| {
        let mut set = FxHashSet::default();
        set.insert("setUp");
        set.insert("tearDown");
        set.insert("setUpClass");
        set.insert("tearDownClass");
        set.insert("setUpModule");
        set.insert("tearDownModule");
        set
    })
}

/// Returns default folders excluded from scanning.
pub fn get_default_exclude_folders() -> &'static FxHashSet<&'static str> {
    static SET: OnceLock<FxHashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| {
        let mut set = FxHashSet::default();
        for folder in [
            "__pycache__",
            ".pytest_cache",
            ".mypy_cache",
            ".ruff_cache",
            ".tox",
            "htmlcov",
            ".coverage",
            "*.egg-info",
            ".eggs",
            "venv",
            ".venv",
            "env",
            ".env",
            ".nox",
            ".pytype",
            "build",
            "dist",
            "site-packages",
            "node_modules",
            ".npm",
            "bower_components",
            "target",
            "vendor",
            ".bundle",
            ".gradle",
            "gradle",
            ".mvn",
            ".git",
            ".svn",
            ".hg",
            ".idea",
            ".vscode",
            ".vs",
            ".cache",
            ".tmp",
            "tmp",
            "logs",
        ] {
            set.insert(folder);
        }
        set
    })
}

/// Returns fixture names provided by pytest core/plugins.
pub fn get_pytest_plugin_fixtures() -> &'static FxHashSet<&'static str> {
    static SET: OnceLock<FxHashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| {
        let mut set = FxHashSet::default();
        for fixture in [
            "request",
            "pytestconfig",
            "tmp_path",
            "tmp_path_factory",
            "tmpdir",
            "tmpdir_factory",
            "capsys",
            "capfd",
            "capsysbinary",
            "capfdbinary",
            "caplog",
            "monkeypatch",
            "recwarn",
            "cache",
            "doctest_namespace",
            "mocker",
            "mock",
            "class_mocker",
            "module_mocker",
            "session_mocker",
            "client",
            "rf",
            "admin_client",
            "admin_user",
            "db",
            "transactional_db",
            "django_db_setup",
            "django_db_blocker",
            "live_server",
            "settings",
            "django_user_model",
            "event_loop",
            "event_loop_policy",
            "httpx_mock",
            "aiohttp_client",
            "app",
            "_push_request_context",
            "freezer",
        ] {
            set.insert(fixture);
        }
        set
    })
}
