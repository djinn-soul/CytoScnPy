//! Shared fixture for cross-format integration tests.

#![allow(dead_code)]

use cytoscnpy::analyzer::AnalysisResult;
use cytoscnpy::clones::{CloneFinding, CloneRelation, CloneType, NodeKind};
use cytoscnpy::deps::{
    DeclaredDependency, DependencyImportLocation, DependencySource, DevDependencyInProduction,
    MissingDependency, TransitiveDependency,
};
use std::path::PathBuf;

pub(super) fn extended_result() -> AnalysisResult {
    let source_location = DependencyImportLocation {
        file: PathBuf::from("src/app.py"),
        line: 7,
        column: 3,
    };
    let mut result = AnalysisResult::default();
    result.clones.push(CloneFinding {
        rule_id: "CSP-C100".to_owned(),
        message: "Exact duplicate implementation".to_owned(),
        severity: "WARNING".to_owned(),
        file: PathBuf::from("src/clone.py"),
        line: 10,
        end_line: 20,
        start_byte: 100,
        end_byte: 200,
        clone_type: CloneType::Type1,
        similarity: 1.0,
        name: Some("duplicate".to_owned()),
        related_clone: CloneRelation {
            file: PathBuf::from("src/original.py"),
            line: 1,
            end_line: 11,
            name: Some("original".to_owned()),
        },
        fix_confidence: 90,
        is_duplicate: true,
        suggestion: None,
        node_kind: NodeKind::Function,
    });
    result.missing_dependencies = vec!["missing_pkg".to_owned()];
    result.missing_dependency_details = vec![MissingDependency {
        import_name: "missing_pkg".to_owned(),
        locations: vec![source_location.clone()],
    }];
    result.unused_dependencies = vec![declared("unused-pkg", false)];
    result.transitive_dependencies = vec![TransitiveDependency {
        import_name: "transitive_pkg".to_owned(),
        package_name: "provider-pkg".to_owned(),
        locations: vec![source_location.clone()],
    }];
    result.dev_dependencies_in_production = vec![DevDependencyInProduction {
        import_name: "pytest".to_owned(),
        dependency: declared("pytest", true),
        locations: vec![source_location],
    }];
    result.stdlib_dependencies = vec![declared("pathlib", false)];
    result
}

fn declared(package_name: &str, is_dev: bool) -> DeclaredDependency {
    DeclaredDependency {
        package_name: package_name.to_owned(),
        normalized_name: package_name.replace('-', "_"),
        is_dev,
        is_optional: false,
        marker: None,
        source: DependencySource::Pyproject,
    }
}
