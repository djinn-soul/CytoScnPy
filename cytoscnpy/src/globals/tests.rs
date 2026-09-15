use std::path::{Path, PathBuf};

use super::polyglot::{detect_js_globals, detect_rust_globals};
use super::python::detect_python_globals;
use super::reporter::{print_json_report, print_terminal_report};
use super::scanner::is_test_file;
use super::types::{GlobalKind, GlobalsResult};

#[test]
fn test_python_module_collections() {
    let code = r"
ITEMS = []
CACHE = {}
UNIQUE = set()
REGISTRY = defaultdict(list)
CONFIG = dict()
";
    let matches = detect_python_globals(code, Path::new("app.py"));
    assert_eq!(matches.len(), 5);
    for m in &matches {
        assert_eq!(m.kind, GlobalKind::ModuleCollection);
    }
    let names: Vec<_> = matches.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["ITEMS", "CACHE", "UNIQUE", "REGISTRY", "CONFIG"]
    );
}

#[test]
fn test_python_immutable_globals_not_flagged() {
    let code = r#"
MAX_RETRIES = 5
BASE_URL = "https://api.example.com"
IS_ENABLED = True
NOTHING = None
DIMENSIONS = (1920, 1080)
IMMUTABLE_SET = frozenset([1, 2, 3])
"#;
    let matches = detect_python_globals(code, Path::new("constants.py"));
    assert!(
        matches.is_empty(),
        "Immutable constants should not be flagged as mutable globals"
    );
}

#[test]
fn test_python_class_mutable_variables() {
    let code = r"
class DataStore:
    items = []
    lookup = {}
    total: int = 0
";
    let matches = detect_python_globals(code, Path::new("store.py"));
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].name, "DataStore::items");
    assert_eq!(matches[0].kind, GlobalKind::ClassVariable);
    assert_eq!(matches[1].name, "DataStore::lookup");
    assert_eq!(matches[1].kind, GlobalKind::ClassVariable);
}

#[test]
fn test_python_function_global_mutation() {
    let code = r"
COUNTER = 0

def increment():
    global COUNTER
    COUNTER += 1

def reset():
    global COUNTER
    COUNTER = 0
";
    let matches = detect_python_globals(code, Path::new("counter.py"));
    assert_eq!(matches.len(), 2);
    for m in &matches {
        assert_eq!(m.kind, GlobalKind::GlobalMutation);
        assert_eq!(m.name, "COUNTER");
    }
}

#[test]
fn test_python_del_global_and_comprehension() {
    let code = r#"
COMP_LIST = [x * 2 for x in range(5)]
COMP_DICT = {k: v for k, v in [("a", 1)]}

def clear_cache():
    global CACHE
    del CACHE

if "__main__" == __name__:
    RUNNER_LIST = [1, 2, 3]
"#;
    let matches = detect_python_globals(code, Path::new("advanced.py"));
    assert_eq!(matches.len(), 3);
    assert_eq!(matches[0].name, "COMP_LIST");
    assert_eq!(matches[1].name, "COMP_DICT");
    assert_eq!(matches[2].name, "CACHE");
    assert_eq!(matches[2].kind, GlobalKind::GlobalMutation);
}

#[test]
fn test_rust_static_mut_detected() {
    let code = r#"
pub static mut GLOBAL_STATE: i32 = 0;
static mut BUFFER: [u8; 1024] = [0; 1024];
const SAFE_CONST: i32 = 42;
static SAFE_STATIC: &str = "hello";
"#;
    let matches = detect_rust_globals(code, Path::new("lib.rs"));
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].name, "GLOBAL_STATE");
    assert_eq!(matches[0].kind, GlobalKind::RustStaticMut);
    assert_eq!(matches[1].name, "BUFFER");
    assert_eq!(matches[1].kind, GlobalKind::RustStaticMut);
}

#[test]
fn test_js_top_level_mutables_detected() {
    let code = r"
var globalCounter = 0;
let userList = [];
let cacheMap = {};
const SAFE_CONST = 100;

function localScope() {
    var localVar = 10;
    let localList = [];
}
";
    let matches = detect_js_globals(code, Path::new("index.js"));
    assert_eq!(matches.len(), 3);
    assert_eq!(matches[0].name, "globalCounter");
    assert_eq!(matches[1].name, "userList");
    assert_eq!(matches[2].name, "cacheMap");
}

#[test]
fn test_is_test_file_policy() {
    assert!(is_test_file(Path::new("tests/test_api.py")));
    assert!(is_test_file(Path::new("src/test_utils.py")));
    assert!(is_test_file(Path::new("src/app_test.py")));
    assert!(is_test_file(Path::new("conftest.py")));
    assert!(is_test_file(Path::new("test/test_db.js")));
    assert!(is_test_file(Path::new("client.test.ts")));
    assert!(!is_test_file(Path::new("src/models/user.py")));
    assert!(!is_test_file(Path::new("src/services/api.rs")));
}

#[test]
fn test_reporter_clean_output() {
    let res = GlobalsResult::new(PathBuf::from("."));
    let mut out = Vec::new();
    print_terminal_report(&res, None, &mut out).expect("Terminal report should succeed");
    let text = String::from_utf8(out).expect("UTF-8 text expected");
    assert!(text.contains("No mutable global state detected"));
}

#[test]
fn test_json_report_serialization() {
    let mut res = GlobalsResult::new(PathBuf::from("."));
    res.stats.total_globals = 1;
    res.stats.module_collection_count = 1;
    let mut out = Vec::new();
    print_json_report(&res, &mut out).expect("JSON report should succeed");
    let text = String::from_utf8(out).expect("UTF-8 text expected");
    assert!(text.contains(r#""total_globals": 1"#));
    assert!(text.contains(r#""module_collection_count": 1"#));
}

#[test]
fn test_python_async_and_nested_global_mutation() {
    let code = r"
CACHE = {}

async def update_async(k, v):
    global CACHE
    if k not in CACHE:
        CACHE[k] = v

class Handler:
    def reset(self):
        global CACHE
        CACHE = {}
";
    let matches = detect_python_globals(code, Path::new("cache.py"));
    assert_eq!(matches.len(), 3);
    assert_eq!(matches[0].kind, GlobalKind::ModuleCollection);
    assert_eq!(matches[1].kind, GlobalKind::GlobalMutation);
    assert_eq!(matches[2].kind, GlobalKind::GlobalMutation);
}
