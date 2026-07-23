//! Regression tests for complete `GitHub` Actions annotation output.

#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::AnalysisResult;
use cytoscnpy::clones::{CloneFinding, CloneRelation, CloneType, NodeKind};
use cytoscnpy::deps::{
    DeclaredDependency, DependencyImportLocation, DependencySource, DevDependencyInProduction,
    MissingDependency, TransitiveDependency,
};
use cytoscnpy::entry_point::run_with_args_to;
use cytoscnpy::report::github;
use cytoscnpy::rules::Finding;
use cytoscnpy::taint::{Severity as TaintSeverity, TaintFinding, VulnType};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn location(line: usize, column: usize) -> DependencyImportLocation {
    DependencyImportLocation {
        file: PathBuf::from("repo/src/app.py"),
        line,
        column,
    }
}

fn declared(name: &str, is_dev: bool, source: DependencySource) -> DeclaredDependency {
    DeclaredDependency {
        package_name: name.to_owned(),
        normalized_name: name.replace('-', "_").to_lowercase(),
        is_dev,
        is_optional: false,
        marker: None,
        source,
    }
}

fn clone_finding() -> CloneFinding {
    CloneFinding {
        rule_id: "CSP-C100".to_owned(),
        message: "Duplicate of original at source.py:2".to_owned(),
        severity: "WARNING".to_owned(),
        file: PathBuf::from("repo/src/duplicate.py"),
        line: 4,
        end_line: 9,
        start_byte: 20,
        end_byte: 80,
        clone_type: CloneType::Type1,
        similarity: 1.0,
        name: Some("duplicate".to_owned()),
        related_clone: CloneRelation {
            file: PathBuf::from("repo/src/source.py"),
            line: 2,
            end_line: 7,
            name: Some("original".to_owned()),
        },
        fix_confidence: 95,
        is_duplicate: true,
        suggestion: None,
        node_kind: NodeKind::Function,
    }
}

#[test]
fn github_report_emits_every_feature_with_valid_coordinates() {
    let mut result = AnalysisResult::default();
    result.danger.push(Finding {
        rule_id: "CSP-D001".to_owned(),
        category: "Security".to_owned(),
        severity: "HIGH".to_owned(),
        message: "Dangerous call".to_owned(),
        file: PathBuf::from("repo/src/app.py"),
        line: 2,
        col: 0,
    });
    result.taint_findings.push(TaintFinding {
        rule_id: "CSP-T001".to_owned(),
        vuln_type: VulnType::SqlInjection,
        severity: TaintSeverity::High,
        file: PathBuf::from("repo/src/app.py"),
        source: "request.args".to_owned(),
        sink: "execute".to_owned(),
        sink_line: 8,
        sink_col: 0,
        source_line: 3,
        flow_path: vec![],
        category: "Taint Analysis".to_owned(),
        remediation: "Use parameters".to_owned(),
        exploitability_score: 90,
    });
    result.clones.push(clone_finding());

    result.missing_dependencies.push("httpx".to_owned());
    result.missing_dependency_details.push(MissingDependency {
        import_name: "httpx".to_owned(),
        locations: vec![location(3, 1)],
    });
    result.missing_dependencies.push("unlocated".to_owned());
    result.transitive_dependencies.push(TransitiveDependency {
        import_name: "yaml".to_owned(),
        package_name: "pyyaml".to_owned(),
        locations: vec![location(4, 2)],
    });
    result
        .dev_dependencies_in_production
        .push(DevDependencyInProduction {
            import_name: "pytest".to_owned(),
            dependency: declared("pytest", true, DependencySource::Pyproject),
            locations: vec![location(5, 3)],
        });
    result.unused_dependencies.push(declared(
        "requests",
        false,
        DependencySource::Requirements("repo/requirements.txt".to_owned()),
    ));
    result
        .stdlib_dependencies
        .push(declared("tomllib", false, DependencySource::Pyproject));

    let mut buffer = Vec::new();
    github::print_github_with_root(&mut buffer, &result, Some(Path::new("repo"))).unwrap();
    let output = String::from_utf8(buffer).unwrap();

    assert!(output.contains("::error file=src/app.py,line=2,col=1,title=CSP-D001::"));
    assert!(output.contains("::error file=src/app.py,line=8,col=1,title=CSP-T001::"));
    assert!(output.contains("::warning file=src/duplicate.py,line=4,endLine=9,title=CSP-C100::"));
    assert!(output.contains("file=src/app.py,line=3,col=1,title=CSP-R001"));
    assert!(output.contains("::error title=CSP-R001::Import 'unlocated'"));
    assert!(output.contains("file=src/app.py,line=4,col=2,title=CSP-R003"));
    assert!(output.contains("file=src/app.py,line=5,col=3,title=CSP-R004"));
    assert!(output.contains("::warning file=requirements.txt,title=CSP-R002::"));
    assert!(output.contains("::warning file=pyproject.toml,title=CSP-R005::"));
    assert!(!output.contains("col=0"));
}

#[test]
fn missing_dependency_evidence_is_serialized_when_present() {
    let mut result = AnalysisResult::default();
    result.missing_dependency_details.push(MissingDependency {
        import_name: "httpx".to_owned(),
        locations: vec![location(3, 1)],
    });

    let json = serde_json::to_value(result).unwrap();
    assert_eq!(
        json["missing_dependency_details"][0]["locations"][0]["line"],
        3
    );
}

#[test]
fn github_cli_preserves_missing_dependency_source_location() -> anyhow::Result<()> {
    let dir = tempdir()?;
    fs::write(
        dir.path().join("pyproject.toml"),
        "[project]\nname = 'github-report-test'\nversion = '0.1.0'\ndependencies = []\n",
    )?;
    fs::write(dir.path().join("main.py"), "import missing_dep\n")?;

    let args = vec![
        dir.path().to_string_lossy().into_owned(),
        "--deps".to_owned(),
        "--format".to_owned(),
        "github".to_owned(),
    ];
    let mut buffer = Vec::new();
    let exit_code = run_with_args_to(args, &mut buffer)?;
    let output = String::from_utf8(buffer)?;

    assert_eq!(exit_code, 0);
    assert!(output.contains("file=main.py,line=1,col=1,title=CSP-R001"));
    Ok(())
}
