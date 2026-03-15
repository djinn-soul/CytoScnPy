//! Regression tests for protocol signature handling.
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
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
