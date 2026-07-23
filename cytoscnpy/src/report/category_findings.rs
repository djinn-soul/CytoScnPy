use crate::analyzer::AnalysisResult;
use crate::deps::{DeclaredDependency, DependencyImportLocation, DependencySource};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(crate) struct ExtendedFinding {
    pub rule_id: String,
    pub kind: &'static str,
    pub category: &'static str,
    pub severity: String,
    pub message: String,
    pub file: Option<PathBuf>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub end_line: Option<usize>,
}

impl ExtendedFinding {
    pub(crate) fn normalized_path(&self, root: Option<&Path>) -> Option<String> {
        self.file.as_deref().map(|path| normalize_path(path, root))
    }

    pub(crate) fn stable_id(&self) -> String {
        let file = self
            .file
            .as_deref()
            .map(crate::utils::normalize_display_path)
            .unwrap_or_else(|| "-".to_owned());
        format!(
            "{}:{}:{}:{}:{}:{}",
            self.kind,
            self.rule_id,
            file,
            self.line.unwrap_or(0),
            self.column.unwrap_or(0),
            self.message
        )
    }

    pub(crate) fn location_or_manifest(&self, root: Option<&Path>) -> (String, usize) {
        (
            self.normalized_path(root)
                .unwrap_or_else(|| "pyproject.toml".to_owned()),
            self.line.unwrap_or(1),
        )
    }
}

pub(crate) fn collect_extended_findings(result: &AnalysisResult) -> Vec<ExtendedFinding> {
    let mut findings = collect_clone_findings(result);
    findings.extend(collect_dependency_findings(result));
    findings
}

pub(crate) fn collect_clone_findings(result: &AnalysisResult) -> Vec<ExtendedFinding> {
    result
        .clones
        .iter()
        .map(|finding| ExtendedFinding {
            rule_id: finding.rule_id.clone(),
            kind: "clone",
            category: "Clone",
            severity: finding.severity.clone(),
            message: finding.message.clone(),
            file: Some(finding.file.clone()),
            line: Some(finding.line),
            column: None,
            end_line: Some(finding.end_line),
        })
        .collect()
}

pub(crate) fn collect_dependency_findings(result: &AnalysisResult) -> Vec<ExtendedFinding> {
    let mut findings = Vec::new();
    add_missing_findings(&mut findings, result);

    for dependency in &result.transitive_dependencies {
        let message = format!(
            "Import '{}' is provided only by transitive dependency '{}'; declare it directly",
            dependency.import_name, dependency.package_name
        );
        add_locations(
            &mut findings,
            "CSP-R003",
            "transitive_dependency",
            "HIGH",
            message,
            &dependency.locations,
        );
    }
    for dependency in &result.dev_dependencies_in_production {
        let message = format!(
            "Development dependency '{}' is imported by production code as '{}'",
            dependency.dependency.package_name, dependency.import_name
        );
        add_locations(
            &mut findings,
            "CSP-R004",
            "dev_dependency_in_production",
            "HIGH",
            message,
            &dependency.locations,
        );
    }
    for dependency in &result.unused_dependencies {
        findings.push(declared_finding(
            dependency,
            "CSP-R002",
            "unused_dependency",
            "MEDIUM",
            format!(
                "Dependency '{}' is declared but never imported",
                dependency.package_name
            ),
        ));
    }
    for dependency in &result.stdlib_dependencies {
        findings.push(declared_finding(
            dependency,
            "CSP-R005",
            "stdlib_dependency",
            "MEDIUM",
            format!(
                "'{}' is part of Python's standard library and should not be declared",
                dependency.package_name
            ),
        ));
    }
    findings
}

fn add_missing_findings(findings: &mut Vec<ExtendedFinding>, result: &AnalysisResult) {
    for detail in &result.missing_dependency_details {
        add_locations(
            findings,
            "CSP-R001",
            "missing_dependency",
            "HIGH",
            format!(
                "Import '{}' is not declared as a project dependency",
                detail.import_name
            ),
            &detail.locations,
        );
    }
    for name in &result.missing_dependencies {
        if !result
            .missing_dependency_details
            .iter()
            .any(|detail| detail.import_name == *name)
        {
            findings.push(base_finding(
                "CSP-R001",
                "missing_dependency",
                "HIGH",
                format!("Import '{name}' is not declared as a project dependency"),
                None,
            ));
        }
    }
}

fn add_locations(
    findings: &mut Vec<ExtendedFinding>,
    rule_id: &str,
    kind: &'static str,
    severity: &str,
    message: String,
    locations: &[DependencyImportLocation],
) {
    if locations.is_empty() {
        findings.push(base_finding(rule_id, kind, severity, message, None));
        return;
    }
    findings.extend(locations.iter().map(|location| {
        let mut finding = base_finding(rule_id, kind, severity, message.clone(), None);
        finding.file = Some(location.file.clone());
        finding.line = Some(location.line);
        finding.column = Some(location.column);
        finding
    }));
}

fn declared_finding(
    dependency: &DeclaredDependency,
    rule_id: &str,
    kind: &'static str,
    severity: &str,
    message: String,
) -> ExtendedFinding {
    base_finding(
        rule_id,
        kind,
        severity,
        message,
        Some(dependency_source_path(&dependency.source)),
    )
}

fn base_finding(
    rule_id: &str,
    kind: &'static str,
    severity: &str,
    message: String,
    file: Option<PathBuf>,
) -> ExtendedFinding {
    ExtendedFinding {
        rule_id: rule_id.to_owned(),
        kind,
        category: "Dependency",
        severity: severity.to_owned(),
        message,
        file,
        line: None,
        column: None,
        end_line: None,
    }
}

fn dependency_source_path(source: &DependencySource) -> PathBuf {
    match source {
        DependencySource::Pyproject => PathBuf::from("pyproject.toml"),
        DependencySource::Requirements(path) | DependencySource::Setup(path) => PathBuf::from(path),
    }
}

fn normalize_path(path: &Path, root: Option<&Path>) -> String {
    let normalized = root.map_or(path, |root| {
        if root.as_os_str() == "." || root.as_os_str().is_empty() {
            path
        } else {
            path.strip_prefix(root).unwrap_or(path)
        }
    });
    let path = normalized.to_string_lossy().replace('\\', "/");
    path.strip_prefix("./").unwrap_or(&path).to_owned()
}
