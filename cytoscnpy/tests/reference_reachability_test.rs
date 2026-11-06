//! Reachability through references that are not calls.
//!
//! A function or class used as a value (stored in a registry, passed as a
//! default, used as a metaclass, named in an annotation) or defined as a nested
//! function must stay reachable when its user is. Each case is a library
//! package with no module-level calls, where reachability starts from the
//! public API.
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
use tempfile::TempDir;

fn mk_tempdir() -> TempDir {
    let mut target = std::env::current_dir().unwrap();
    target.push("target");
    target.push("test-reference-reach-tmp");
    std::fs::create_dir_all(&target).unwrap();
    tempfile::Builder::new()
        .prefix("ref_reach_")
        .tempdir_in(target)
        .unwrap()
}

/// Names of functions, classes and methods reported as unreachable.
fn unreachable_names(code: &str) -> Vec<String> {
    unreachable_names_in(&[("a.py", code)])
}

/// Same as [`unreachable_names`] for several modules of the package.
fn unreachable_names_in(modules: &[(&str, &str)]) -> Vec<String> {
    let dir = mk_tempdir();
    let package = dir.path().join("lib");
    std::fs::create_dir_all(&package).unwrap();
    std::fs::write(package.join("__init__.py"), "").unwrap();
    for (name, code) in modules {
        std::fs::write(package.join(name), code).unwrap();
    }

    let result = CytoScnPy::default()
        .with_confidence(0)
        .with_tests(false)
        .analyze(dir.path());
    result
        .unused_functions
        .iter()
        .chain(&result.unused_classes)
        .chain(&result.unused_methods)
        .filter(|def| def.is_unreachable)
        .map(|def| def.simple_name.clone())
        .collect()
}

fn assert_reachable(code: &str) {
    let unreachable = unreachable_names(code);
    assert!(
        unreachable.is_empty(),
        "unexpectedly unreachable: {unreachable:?}"
    );
}

#[test]
fn function_in_module_level_registry_is_reachable() {
    assert_reachable(
        r"
def _helper(x):
    return x

_TABLE = {int: _helper}

def public(x):
    return _TABLE[type(x)](x)
",
    );
}

#[test]
fn callee_of_returned_nested_function_is_reachable() {
    assert_reachable(
        r"
def _lookup(x):
    return x

def wrap(app):
    async def wrapped(scope):
        return _lookup(scope)
    return wrapped
",
    );
}

#[test]
fn default_argument_and_metaclass_are_reachable() {
    assert_reachable(
        r"
async def _op(a, b):
    return a + b

async def reduce(fn=_op):
    return await fn(1, 2)

class _Meta(type):
    pass

class Base(metaclass=_Meta):
    pass
",
    );
}

#[test]
fn class_named_only_in_annotations_is_reachable() {
    assert_reachable(
        r#"
import typing as t

class _Fwd:
    pass

class _Guard:
    pass

def public(x: "_Fwd | None") -> t.TypeGuard[_Guard]:
    return True
"#,
    );
}

#[test]
fn private_hook_of_external_base_is_reachable() {
    // `textwrap.TextWrapper` calls `_handle_long_word`; the subclass shares
    // the base's name, which must not make the base look internal.
    assert_reachable(
        r"
import textwrap

def _cut(chunks):
    return chunks[:1]

class TextWrapper(textwrap.TextWrapper):
    def _handle_long_word(self, chunks, cur, cur_len, width):
        return _cut(chunks)
",
    );
}

#[test]
fn dead_chains_stay_unreachable() {
    let mut unreachable = unreachable_names(
        r"
def _dead():
    pass

def _also_dead():
    _dead()

def _cut(s):
    return s

class Base:
    def run(self):
        return 1

class Child(Base):
    def _priv(self):
        return _cut(1)
",
    );
    unreachable.sort();
    assert_eq!(
        unreachable,
        ["_also_dead", "_cut", "_dead", "_priv"].map(str::to_owned)
    );
}

#[test]
fn function_referenced_only_from_dead_code_is_unreachable() {
    let mut unreachable = unreachable_names(
        r"
def _dead_cb():
    pass

def _dead_user():
    return [_dead_cb]

def _live_cb():
    pass

def public():
    return [_live_cb]
",
    );
    unreachable.sort();
    assert_eq!(unreachable, ["_dead_cb", "_dead_user"].map(str::to_owned));
}

#[test]
fn property_getter_calls_survive_its_setter() {
    // The setter shares the getter's name; the getter's calls must remain.
    assert_reachable(
        r"
def _is_known(value):
    return True

class Response:
    @property
    def encoding(self):
        return _is_known(1)

    @encoding.setter
    def encoding(self, value):
        pass

__all__ = ['Response']
",
    );
}

#[test]
fn overload_stub_annotations_survive_the_implementation() {
    assert_reachable(
        r"
from typing import TypedDict, Unpack, overload

class _Options(TypedDict):
    pass

class Session:
    @overload
    def request(self, **kwargs: Unpack[_Options]) -> None: ...
    def request(self, **kwargs):
        pass

__all__ = ['Session']
",
    );
}

#[test]
fn call_through_module_attribute_is_reachable() {
    let unreachable = unreachable_names_in(&[
        (
            "a.py",
            r"
def _set_proxied(c):
    return c

def _bootstrap(cls):
    cls.proxied = property(_set_proxied)
",
        ),
        (
            "manager.py",
            r"
from . import a

def boot(cls):
    a._bootstrap(cls)
",
        ),
    ]);
    assert!(
        unreachable.is_empty(),
        "unexpectedly unreachable: {unreachable:?}"
    );
}
