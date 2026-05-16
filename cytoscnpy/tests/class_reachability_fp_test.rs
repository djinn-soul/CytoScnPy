//! Regression tests for class-level unreachable false positives.
//!
//! These tests exercise the full multi-file pipeline (which is where
//! reachability/classification runs) — `analyze_code()` bypasses that path
//! and cannot observe `is_unreachable`.
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
fn base_class_of_reachable_subclass_is_not_unreachable() {
    let dir = mk_tempdir("base_class_");
    write_file(
        dir.path(),
        "main.py",
        r"
class Base:
    def helper(self):
        return 1

class Derived(Base):
    def run(self):
        return self.helper()

Derived().run()
",
    );

    let result = app_analyzer().analyze(dir.path());

    let unreachable: Vec<_> = result
        .unused_classes
        .iter()
        .filter(|d| d.is_unreachable)
        .map(|d| d.simple_name.as_str())
        .collect();

    assert!(
        !unreachable.contains(&"Base"),
        "Base class of reachable subclass should not be unreachable; got {unreachable:?}"
    );
}

#[test]
fn nested_class_inside_reachable_method_is_not_unreachable() {
    let dir = mk_tempdir("nested_class_");
    write_file(
        dir.path(),
        "main.py",
        r"
class Outer:
    def factory(self):
        class Inner:
            def value(self):
                return 7
        return Inner()

Outer().factory()
",
    );

    let result = app_analyzer().analyze(dir.path());

    let unreachable: Vec<_> = result
        .unused_classes
        .iter()
        .filter(|d| d.is_unreachable)
        .map(|d| d.simple_name.as_str())
        .collect();

    assert!(
        !unreachable.contains(&"Inner"),
        "Inner class nested inside reachable method should not be unreachable; got {unreachable:?}"
    );
}

#[test]
fn class_referenced_only_via_attribute_is_not_unreachable() {
    let dir = mk_tempdir("attr_class_");
    write_file(
        dir.path(),
        "main.py",
        r"
class Helpers:
    pass

class Backend:
    def serve(self):
        return 1

Helpers.Backend = Backend

def setup():
    return Helpers.Backend()

setup()
",
    );

    let result = app_analyzer().analyze(dir.path());

    let unreachable: Vec<_> = result
        .unused_classes
        .iter()
        .filter(|d| d.is_unreachable)
        .map(|d| d.simple_name.as_str())
        .collect();

    assert!(
        !unreachable.contains(&"Backend"),
        "Backend referenced via attribute should not be unreachable; got {unreachable:?}"
    );
}

#[test]
fn truly_dead_class_is_still_reported() {
    let dir = mk_tempdir("dead_class_");
    write_file(
        dir.path(),
        "main.py",
        r"
class Used:
    def go(self):
        return 1

class Orphan:
    def lonely(self):
        return 2

Used().go()
",
    );

    let result = app_analyzer().analyze(dir.path());

    let unreachable: Vec<_> = result
        .unused_classes
        .iter()
        .filter(|d| d.is_unreachable)
        .map(|d| d.simple_name.as_str())
        .collect();

    assert!(
        unreachable.contains(&"Orphan"),
        "Genuinely dead class must still be flagged unreachable; got {unreachable:?}"
    );
}
