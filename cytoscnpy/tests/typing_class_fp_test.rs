//! Tests for class in typing false positives.
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
use cytoscnpy::config::ProjectType;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

fn mk_tempdir(prefix: &str) -> TempDir {
    let mut target = std::env::current_dir().unwrap();
    target.push("target");
    target.push("test-class-reach-tmp");
    std::fs::create_dir_all(&target).unwrap();
    tempfile::Builder::new()
        .prefix(prefix)
        .tempdir_in(target)
        .unwrap()
}

fn app_analyzer() -> CytoScnPy {
    let mut a = CytoScnPy::default().with_confidence(60).with_tests(false);
    a.config.cytoscnpy.project_type = Some(ProjectType::Application);
    a
}

fn write_file(dir: &std::path::Path, name: &str, body: &str) {
    let mut f = File::create(dir.join(name)).unwrap();
    writeln!(f, "{body}").unwrap();
}

#[test]
fn test_app_class_in_type_annotation() {
    let dir = mk_tempdir("ann_class_");
    write_file(
        dir.path(),
        "main.py",
        r"
class TargetClass:
    pass

def process(x: TargetClass) -> None:
    pass

process(None)
",
    );
    let result = app_analyzer().analyze(dir.path());
    let unused: Vec<_> = result
        .unused_classes
        .iter()
        .map(|c| c.simple_name.as_str())
        .collect();
    assert!(
        !unused.contains(&"TargetClass"),
        "TargetClass should not be reported unused"
    );
}

#[test]
fn test_app_class_in_class_attribute_annotation() {
    let dir = mk_tempdir("cls_ann_class_");
    write_file(
        dir.path(),
        "main.py",
        r"
class TargetClass:
    pass

class Container:
    item: TargetClass

Container()
",
    );
    let result = app_analyzer().analyze(dir.path());
    let unused: Vec<_> = result
        .unused_classes
        .iter()
        .map(|c| c.simple_name.as_str())
        .collect();
    assert!(
        !unused.contains(&"TargetClass"),
        "TargetClass should not be reported unused"
    );
}

#[test]
fn test_app_class_in_generic_base() {
    let dir = mk_tempdir("gen_base_");
    write_file(
        dir.path(),
        "main.py",
        r#"
from typing import Generic, TypeVar

T = TypeVar("T")

class Base(Generic[T]):
    pass

class TargetClass:
    pass

class Concrete(Base[TargetClass]):
    pass

Concrete()
"#,
    );
    let result = app_analyzer().analyze(dir.path());
    let unused: Vec<_> = result
        .unused_classes
        .iter()
        .map(|c| c.simple_name.as_str())
        .collect();
    assert!(
        !unused.contains(&"TargetClass"),
        "TargetClass should not be reported unused"
    );
}

#[test]
fn test_app_class_in_typed_dict() {
    let dir = mk_tempdir("typed_dict_");
    write_file(
        dir.path(),
        "main.py",
        r"
import typing

class TargetClass:
    pass

class MyTypedDict(typing.TypedDict):
    item: TargetClass

def use_dict(d: MyTypedDict) -> None:
    pass

use_dict({})
",
    );
    let result = app_analyzer().analyze(dir.path());
    let unused: Vec<_> = result
        .unused_classes
        .iter()
        .map(|c| c.simple_name.as_str())
        .collect();
    assert!(
        !unused.contains(&"TargetClass"),
        "TargetClass should not be reported unused"
    );
}

#[test]
fn test_app_class_in_type_alias() {
    let dir = mk_tempdir("type_alias_");
    write_file(
        dir.path(),
        "main.py",
        r"
class TargetClass:
    pass

type MyAlias = TargetClass

def use_alias(x: MyAlias) -> None:
    pass

use_alias(None)
",
    );
    let result = app_analyzer().analyze(dir.path());
    let unused: Vec<_> = result
        .unused_classes
        .iter()
        .map(|c| c.simple_name.as_str())
        .collect();
    assert!(
        !unused.contains(&"TargetClass"),
        "TargetClass should not be reported unused"
    );
}

#[test]
fn test_cross_file_class_in_typing_class() {
    let dir = mk_tempdir("cross_file_typing_");
    write_file(
        dir.path(),
        "item_types.py",
        r"
class TargetClass:
    pass
",
    );
    write_file(
        dir.path(),
        "models.py",
        r"
from item_types import TargetClass
import typing

class MyTypedDict(typing.TypedDict):
    item: TargetClass
",
    );
    write_file(
        dir.path(),
        "main.py",
        r"
from models import MyTypedDict

def run(d: MyTypedDict) -> None:
    pass

run({})
",
    );
    let result = app_analyzer().analyze(dir.path());
    let unused: Vec<_> = result
        .unused_classes
        .iter()
        .map(|c| c.simple_name.as_str())
        .collect();
    assert!(
        !unused.contains(&"TargetClass"),
        "TargetClass should not be reported unused across files"
    );
}

#[test]
fn test_dead_class_in_unreachable_typing_class_is_reported() {
    let dir = mk_tempdir("dead_typing_");
    write_file(
        dir.path(),
        "main.py",
        r"
import typing

class TrulyDeadTarget:
    pass

class UnusedDict(typing.TypedDict):
    item: TrulyDeadTarget

class UsedClass:
    pass

UsedClass()
",
    );
    let result = app_analyzer().analyze(dir.path());
    let unused: Vec<_> = result
        .unused_classes
        .iter()
        .map(|c| c.simple_name.as_str())
        .collect();
    assert!(
        unused.contains(&"TrulyDeadTarget"),
        "Truly dead class should still be reported: {unused:?}"
    );
    assert!(
        unused.contains(&"UnusedDict"),
        "Unused TypedDict should be reported: {unused:?}"
    );
    assert!(
        !unused.contains(&"UsedClass"),
        "UsedClass should not be reported: {unused:?}"
    );
}
