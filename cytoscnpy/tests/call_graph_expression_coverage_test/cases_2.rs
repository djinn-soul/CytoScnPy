use super::*;

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
