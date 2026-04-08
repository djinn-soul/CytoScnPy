//! Regression tests for protocol signature handling.
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
use cytoscnpy::config::ProjectType;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-protocol-signature-tmp");
    std::fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("protocol_signature_")
        .tempdir_in(target_dir)
        .unwrap()
}

#[test]
fn test_generic_protocol_signatures_are_not_reported_unused() {
    let dir = project_tempdir();
    let file_path = dir.path().join("protocols.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
from typing import Protocol, TypeVar

T = TypeVar("T")

class Repository(Protocol[T]):
    def get_by_id(self, id: int) -> T: ...

class ConcreteRepo:
    def get_by_id(self, id: int) -> int:
        return id
"#
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = analyzer.analyze(dir.path());

    let protocol_method = result
        .unused_methods
        .iter()
        .find(|d| d.full_name == "protocols.Repository.get_by_id");
    assert!(
        protocol_method.is_none(),
        "Protocol method signatures should not be reported as unused methods"
    );

    let protocol_param = result
        .unused_parameters
        .iter()
        .find(|d| d.full_name == "protocols.Repository.get_by_id.id");
    assert!(
        protocol_param.is_none(),
        "Protocol method parameters should not be reported as unused parameters"
    );
}

#[test]
fn test_runtime_checkable_protocol_isinstance_marks_required_method_as_used() {
    let dir = project_tempdir();
    let file_path = dir.path().join("runtime_protocol.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
from typing import Protocol, runtime_checkable

@runtime_checkable
class Runner(Protocol):
    def run(self) -> str: ...

class EchoRunner:
    def run(self) -> str:
        return "ok"

def accepts_runtime_checked(value: object) -> bool:
    return isinstance(value, Runner)

accepts_runtime_checked(EchoRunner())
"#
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(0).with_tests(false);
    analyzer.config.cytoscnpy.project_type = Some(ProjectType::Application);
    let result = analyzer.analyze(dir.path());

    let echo_run_method = result
        .unused_methods
        .iter()
        .find(|d| d.full_name == "runtime_protocol.EchoRunner.run");
    assert!(
        echo_run_method.is_none(),
        "Runtime protocol checks should keep required implementing methods reachable"
    );
}

#[test]
fn test_casted_protocol_in_function_flow_is_not_reported_unused() {
    let dir = project_tempdir();
    let file_path = dir.path().join("cast_protocol_flow.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
from typing import Protocol, cast, runtime_checkable

@runtime_checkable
class Runner(Protocol):
    def run(self) -> str: ...

class Impl:
    def run(self) -> str:
        return "ok"

def call_runner(value: object) -> str:
    if not isinstance(value, Runner):
        return "nope"
    runner = cast(Runner, value)
    return runner.run()

call_runner(Impl())
"#
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(0).with_tests(false);
    analyzer.config.cytoscnpy.project_type = Some(ProjectType::Application);
    let result = analyzer.analyze(dir.path());

    let impl_run_method = result
        .unused_methods
        .iter()
        .find(|d| d.full_name == "cast_protocol_flow.Impl.run");
    assert!(
        impl_run_method.is_none(),
        "Protocol + cast flow in function scope should keep implementation method reachable"
    );
}

#[test]
fn test_runtime_checkable_protocol_cast_only_flow_is_not_reported_unused() {
    let dir = project_tempdir();
    let file_path = dir.path().join("cast_only_runtime_protocol.py");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
from typing import Protocol, cast, runtime_checkable

@runtime_checkable
class Runner(Protocol):
    def run(self) -> str: ...

class Impl:
    def run(self) -> str:
        return "ok"

def call_runner(value: object) -> str:
    runner = cast(Runner, value)
    return runner.run()

call_runner(Impl())
"#
    )
    .unwrap();

    let mut analyzer = CytoScnPy::default().with_confidence(0).with_tests(false);
    analyzer.config.cytoscnpy.project_type = Some(ProjectType::Application);
    let result = analyzer.analyze(dir.path());

    let impl_run_method = result
        .unused_methods
        .iter()
        .find(|d| d.full_name == "cast_only_runtime_protocol.Impl.run");
    assert!(
        impl_run_method.is_none(),
        "Cast-only runtime-checkable protocol flow should keep implementation method reachable"
    );
}
