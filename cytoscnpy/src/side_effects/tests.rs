//! Unit tests for module-level side-effects analysis.
#![allow(clippy::unwrap_used)]

use std::path::Path;

use super::polyglot::detect_js_side_effects;
use super::python::detect_python_side_effects;
use super::types::SideEffectKind;

fn py_path() -> &'static Path {
    Path::new("module.py")
}

fn js_path() -> &'static Path {
    Path::new("index.ts")
}

// ── Python tests ─────────────────────────────────────────────────────────

#[test]
fn test_python_toplevel_call_detected() {
    let src = "def helper():\n    pass\n\ninit_database('prod.db')\n";
    let matches = detect_python_side_effects(src, py_path());
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].kind, SideEffectKind::PythonTopLevelCall);
    assert_eq!(matches[0].line, 4);
}

#[test]
fn test_python_safe_calls_not_flagged() {
    let src = concat!(
        "import logging\n",
        "import warnings\n",
        "logging.basicConfig(level=logging.INFO)\n",
        "logger = logging.getLogger(__name__)\n",
        "logger.info('module loaded')\n",
        "warnings.filterwarnings('ignore')\n",
        "print('debug info')\n",
        "def main():\n",
        "    pass\n",
    );
    let matches = detect_python_side_effects(src, py_path());
    assert!(
        matches.is_empty(),
        "logging/warnings/print setup should not be flagged: {matches:?}"
    );
}

#[test]
fn test_python_main_guard_ignored() {
    let src = concat!(
        "def run():\n",
        "    pass\n",
        "if __name__ == '__main__':\n",
        "    init_database()\n",
        "    start_server()\n",
    );
    let matches = detect_python_side_effects(src, py_path());
    assert!(
        matches.is_empty(),
        "calls inside if __name__ == '__main__' should be ignored"
    );
}

#[test]
fn test_python_type_checking_guard_ignored() {
    let src = concat!(
        "from typing import TYPE_CHECKING\n",
        "if TYPE_CHECKING:\n",
        "    from expensive_module import HeavyType\n",
    );
    let matches = detect_python_side_effects(src, py_path());
    assert!(matches.is_empty());
}

#[test]
fn test_python_toplevel_loops_and_with_detected() {
    let src = concat!(
        "for i in range(10):\n",
        "    seed_cache(i)\n",
        "with open('config.json') as f:\n",
        "    data = f.read()\n",
    );
    let matches = detect_python_side_effects(src, py_path());
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].kind, SideEffectKind::PythonTopLevelLoop);
    assert_eq!(matches[0].line, 1);
    assert_eq!(matches[1].kind, SideEffectKind::PythonTopLevelWith);
    assert_eq!(matches[1].line, 3);
}

#[test]
fn test_python_exempt_files() {
    let src = "setup(name='mypkg')\n";
    let matches = detect_python_side_effects(src, Path::new("setup.py"));
    assert!(matches.is_empty(), "setup.py should be exempt");

    let matches_conftest = detect_python_side_effects(src, Path::new("conftest.py"));
    assert!(matches_conftest.is_empty(), "conftest.py should be exempt");
}

// ── JavaScript / TypeScript tests ────────────────────────────────────────

#[test]
fn test_js_toplevel_event_listener_detected() {
    let src = "window.addEventListener('load', () => {\n  init();\n});\n";
    let matches = detect_js_side_effects(src, js_path());
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].kind, SideEffectKind::JsEventListener);
    assert_eq!(matches[0].line, 1);
}

#[test]
fn test_js_toplevel_server_call_detected() {
    let src = "const app = express();\napp.use(cors());\napp.get('/api', handler);\n";
    let matches = detect_js_side_effects(src, js_path());
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].kind, SideEffectKind::JsTopLevelCall);
    assert_eq!(matches[0].line, 2);
    assert_eq!(matches[1].kind, SideEffectKind::JsTopLevelCall);
    assert_eq!(matches[1].line, 3);
}

#[test]
fn test_js_inside_function_not_flagged() {
    let src = concat!(
        "function setup() {\n",
        "  window.addEventListener('resize', handleResize);\n",
        "  fetch('/health');\n",
        "}\n",
    );
    let matches = detect_js_side_effects(src, js_path());
    assert!(
        matches.is_empty(),
        "calls inside function body should not be flagged as top-level"
    );
}

#[test]
fn test_js_single_line_function_not_flagged() {
    let src = "function setup() { app.use('/api', router); }\n";
    let matches = detect_js_side_effects(src, js_path());
    assert!(
        matches.is_empty(),
        "single-line function calls must not be flagged as top-level"
    );
}
