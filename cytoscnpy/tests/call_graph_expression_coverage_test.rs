//! Tests that the call graph correctly records calls made inside every
//! expression form and statement form that was previously missed.
//!
//! Each test:
//!  - Defines a helper function that is called ONLY through one specific
//!    expression or statement construct.
//!  - Asserts that the helper is NOT reported as unreachable (the call graph
//!    must have picked up the edge, otherwise it would be flagged).
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
use cytoscnpy::config::ProjectType;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

fn mk_dir(prefix: &str) -> TempDir {
    let mut base = std::env::current_dir().unwrap();
    base.push("target");
    base.push("test-cg-expr-tmp");
    std::fs::create_dir_all(&base).unwrap();
    tempfile::Builder::new()
        .prefix(prefix)
        .tempdir_in(base)
        .unwrap()
}

fn app_analyzer() -> CytoScnPy {
    let mut a = CytoScnPy::default().with_confidence(60).with_tests(false);
    a.config.cytoscnpy.project_type = Some(ProjectType::Application);
    a
}

fn unreachable_fns(result: &cytoscnpy::analyzer::types::AnalysisResult) -> Vec<String> {
    result
        .unused_functions
        .iter()
        .filter(|d| d.is_unreachable)
        .map(|d| d.simple_name.clone())
        .collect()
}

// ── helpers ─────────────────────────────────────────────────────────────────

fn write_py(dir: &TempDir, name: &str, code: &str) {
    let mut f = File::create(dir.path().join(name)).unwrap();
    writeln!(f, "{code}").unwrap();
}

// ── BoolOp (`a and b`) ──────────────────────────────────────────────────────
#[test]
fn fn_called_in_bool_op_not_unreachable() {
    let dir = mk_dir("cg_boolop_");
    write_py(
        &dir,
        "app.py",
        r"
def _check_a():
    return True

def _check_b():
    return True

def run():
    return _check_a() and _check_b()

run()
",
    );
    let result = app_analyzer().analyze(dir.path());
    let ur = unreachable_fns(&result);
    assert!(!ur.contains(&"_check_a".to_owned()), "got {ur:?}");
    assert!(!ur.contains(&"_check_b".to_owned()), "got {ur:?}");
}

// ── UnaryOp (`not f()`) ─────────────────────────────────────────────────────
#[test]
fn fn_called_in_unary_op_not_unreachable() {
    let dir = mk_dir("cg_unary_");
    write_py(
        &dir,
        "app.py",
        r"
def _flag():
    return True

def run():
    return not _flag()

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_flag".to_owned()), "got {ur:?}");
}

// ── Compare (`f() < g()`) ───────────────────────────────────────────────────
#[test]
fn fn_called_in_compare_not_unreachable() {
    let dir = mk_dir("cg_compare_");
    write_py(
        &dir,
        "app.py",
        r"
def _get_x():
    return 1

def _get_y():
    return 2

def run():
    return _get_x() < _get_y()

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_get_x".to_owned()), "got {ur:?}");
    assert!(!ur.contains(&"_get_y".to_owned()), "got {ur:?}");
}

// ── Tuple (`(f(), g())`) ────────────────────────────────────────────────────
#[test]
fn fn_called_inside_tuple_not_unreachable() {
    let dir = mk_dir("cg_tuple_");
    write_py(
        &dir,
        "app.py",
        r"
def _first():
    return 1

def _second():
    return 2

def run():
    return (_first(), _second())

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_first".to_owned()), "got {ur:?}");
    assert!(!ur.contains(&"_second".to_owned()), "got {ur:?}");
}

// ── Set (`{f()}`) ────────────────────────────────────────────────────────────
#[test]
fn fn_called_inside_set_literal_not_unreachable() {
    let dir = mk_dir("cg_set_");
    write_py(
        &dir,
        "app.py",
        r"
def _val():
    return 42

def run():
    return {_val()}

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_val".to_owned()), "got {ur:?}");
}

// ── Dict key containing a call (`{f(): 1}`) ─────────────────────────────────
#[test]
fn fn_called_as_dict_key_not_unreachable() {
    let dir = mk_dir("cg_dict_key_");
    write_py(
        &dir,
        "app.py",
        r"
def _make_key():
    return 'k'

def run():
    return {_make_key(): 1}

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_make_key".to_owned()), "got {ur:?}");
}

// ── ListComp element (`[f(x) for x in lst]`) ────────────────────────────────
#[test]
fn fn_called_in_listcomp_element_not_unreachable() {
    let dir = mk_dir("cg_listcomp_elt_");
    write_py(
        &dir,
        "app.py",
        r"
def _transform(x):
    return x * 2

def run():
    return [_transform(x) for x in range(5)]

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_transform".to_owned()), "got {ur:?}");
}

// ── ListComp iterable (`[x for x in f()]`) ──────────────────────────────────
#[test]
fn fn_called_as_listcomp_iterable_not_unreachable() {
    let dir = mk_dir("cg_listcomp_iter_");
    write_py(
        &dir,
        "app.py",
        r"
def _items():
    return [1, 2, 3]

def run():
    return [x for x in _items()]

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_items".to_owned()), "got {ur:?}");
}

// ── ListComp filter condition (`[x for x in lst if f(x)]`) ──────────────────
#[test]
fn fn_called_in_listcomp_condition_not_unreachable() {
    let dir = mk_dir("cg_listcomp_cond_");
    write_py(
        &dir,
        "app.py",
        r"
def _keep(x):
    return x > 0

def run():
    return [x for x in range(10) if _keep(x)]

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_keep".to_owned()), "got {ur:?}");
}

// ── DictComp (`{k: f(v) for k, v in items}`) ────────────────────────────────
#[test]
fn fn_called_in_dictcomp_not_unreachable() {
    let dir = mk_dir("cg_dictcomp_");
    write_py(
        &dir,
        "app.py",
        r"
def _process(v):
    return v + 1

def run():
    return {k: _process(v) for k, v in enumerate(range(3))}

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_process".to_owned()), "got {ur:?}");
}

// ── Generator expr (`sum(f(x) for x in lst)`) ───────────────────────────────
#[test]
fn fn_called_in_generator_expr_not_unreachable() {
    let dir = mk_dir("cg_genexpr_");
    write_py(
        &dir,
        "app.py",
        r"
def _score(x):
    return x * x

def run():
    return sum(_score(x) for x in range(5))

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_score".to_owned()), "got {ur:?}");
}

// ── Lambda body (`lambda: f()`) ─────────────────────────────────────────────
#[test]
fn fn_called_inside_lambda_body_not_unreachable() {
    let dir = mk_dir("cg_lambda_");
    write_py(
        &dir,
        "app.py",
        r"
def _compute():
    return 99

def run():
    fn = lambda: _compute()
    return fn()

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_compute".to_owned()), "got {ur:?}");
}

// ── Walrus operator (`x := f()`) ────────────────────────────────────────────
#[test]
fn fn_called_via_walrus_operator_not_unreachable() {
    let dir = mk_dir("cg_walrus_");
    write_py(
        &dir,
        "app.py",
        r"
def _get_value():
    return 5

def run():
    if (v := _get_value()) > 0:
        return v
    return 0

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_get_value".to_owned()), "got {ur:?}");
}

// ── Await expression (`await f()`) ──────────────────────────────────────────
#[test]
fn fn_called_via_await_not_unreachable() {
    let dir = mk_dir("cg_await_");
    write_py(
        &dir,
        "app.py",
        r"
import asyncio

async def _fetch():
    return 42

async def run():
    return await _fetch()

asyncio.run(run())
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_fetch".to_owned()), "got {ur:?}");
}

// ── Subscript call (`d[f()]`) ────────────────────────────────────────────────
#[test]
fn fn_called_as_subscript_key_not_unreachable() {
    let dir = mk_dir("cg_subscript_");
    write_py(
        &dir,
        "app.py",
        r"
def _key():
    return 0

def run():
    lst = [10, 20, 30]
    return lst[_key()]

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_key".to_owned()), "got {ur:?}");
}

// ── Starred expression (`f(*get_args())`) ───────────────────────────────────
#[test]
fn fn_called_inside_starred_expr_not_unreachable() {
    let dir = mk_dir("cg_starred_");
    write_py(
        &dir,
        "app.py",
        r"
def _get_args():
    return [1, 2, 3]

def _sum(*args):
    return sum(args)

def run():
    return _sum(*_get_args())

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_get_args".to_owned()), "got {ur:?}");
}

// ── With context_expr (`with acquire() as ctx:`) ────────────────────────────
#[test]
fn fn_called_as_with_context_manager_not_unreachable() {
    let dir = mk_dir("cg_with_ctx_");
    write_py(
        &dir,
        "app.py",
        r"
from contextlib import contextmanager

@contextmanager
def _acquire():
    yield 'resource'

def run():
    with _acquire() as ctx:
        return ctx

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_acquire".to_owned()), "got {ur:?}");
}

// ── Raise statement (`raise Exception(f())`) ────────────────────────────────
#[test]
fn fn_called_in_raise_not_unreachable() {
    let dir = mk_dir("cg_raise_");
    write_py(
        &dir,
        "app.py",
        r"
def _make_error():
    return ValueError('bad')

def run(bad):
    if bad:
        raise _make_error()

run(False)
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_make_error".to_owned()), "got {ur:?}");
}

// ── Match statement subject (`match f():`) ───────────────────────────────────
#[test]
fn fn_called_as_match_subject_not_unreachable() {
    let dir = mk_dir("cg_match_");
    write_py(
        &dir,
        "app.py",
        r"
def _get_cmd():
    return 'start'

def run():
    match _get_cmd():
        case 'start':
            return 1
        case _:
            return 0

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_get_cmd".to_owned()), "got {ur:?}");
}

// ── F-string interpolation (`f'{f()}'`) ─────────────────────────────────────
#[test]
fn fn_called_inside_fstring_not_unreachable() {
    let dir = mk_dir("cg_fstring_");
    write_py(
        &dir,
        "app.py",
        r"
def _name():
    return 'world'

def run():
    return f'Hello {_name()}'

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_name".to_owned()), "got {ur:?}");
}

// ── Slice call (`lst[f():g()]`) ──────────────────────────────────────────────
#[test]
fn fn_called_in_slice_not_unreachable() {
    let dir = mk_dir("cg_slice_");
    write_py(
        &dir,
        "app.py",
        r"
def _start():
    return 1

def _end():
    return 4

def run():
    lst = [0, 1, 2, 3, 4]
    return lst[_start():_end()]

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_start".to_owned()), "got {ur:?}");
    assert!(!ur.contains(&"_end".to_owned()), "got {ur:?}");
}

// ── Match guard (`case x if f(x):`) ─────────────────────────────────────────
#[test]
fn fn_called_in_match_guard_not_unreachable() {
    let dir = mk_dir("cg_match_guard_");
    write_py(
        &dir,
        "app.py",
        r"
def _is_valid(x):
    return x > 0

def run(val):
    match val:
        case x if _is_valid(x):
            return x
        case _:
            return 0

run(5)
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_is_valid".to_owned()), "got {ur:?}");
}

// ── Match class pattern (`case MyClass(attr=expr):`) ─────────────────────────
#[test]
fn fn_called_in_match_class_pattern_not_unreachable() {
    let dir = mk_dir("cg_match_cls_");
    write_py(
        &dir,
        "app.py",
        r"
class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y

def _make_point():
    return Point(1, 2)

def run():
    match _make_point():
        case Point():
            return 'point'
        case _:
            return 'other'

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_make_point".to_owned()), "got {ur:?}");
}
