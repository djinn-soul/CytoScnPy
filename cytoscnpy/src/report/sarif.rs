use crate::analyzer::AnalysisResult;
use serde::Serialize;
use std::io::Write;

#[derive(Serialize)]
struct SarifLog {
    version: String,
    #[serde(rename = "$schema")]
    schema: String,
    runs: Vec<Run>,
}

#[derive(Serialize)]
struct Run {
    tool: Tool,
    results: Vec<SarifResult>,
}

#[derive(Serialize)]
struct Tool {
    driver: Driver,
}

#[derive(Serialize)]
struct Driver {
    name: String,
    version: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    rules: Vec<Rule>,
}

#[derive(Serialize)]
struct Rule {
    id: String,
    #[serde(rename = "shortDescription")]
    short_description: Message,
}

#[derive(Serialize)]
struct SarifResult {
    #[serde(rename = "ruleId")]
    rule_id: String,
    level: String, // "error", "warning", "note", "none"
    message: Message,
    locations: Vec<Location>,
}

#[derive(Serialize)]
struct Message {
    text: String,
}

#[derive(Serialize)]
struct Location {
    #[serde(rename = "physicalLocation")]
    physical_location: PhysicalLocation,
}

#[derive(Serialize)]
struct PhysicalLocation {
    #[serde(rename = "artifactLocation")]
    artifact_location: ArtifactLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    region: Option<Region>,
}

#[derive(Serialize)]
struct ArtifactLocation {
    uri: String,
}

#[derive(Serialize)]
struct Region {
    #[serde(rename = "startLine")]
    start_line: usize,
}

/// Generates a SARIF (Static Analysis Results Interchange Format) report.
///
/// # Errors
///
/// Returns an error if writing to the `writer` fails.
pub fn print_sarif(writer: &mut impl Write, result: &AnalysisResult) -> std::io::Result<()> {
    print_sarif_with_root(writer, result, None)
}

/// Generates a SARIF (Static Analysis Results Interchange Format) report with an optional root path.
///
/// # Errors
///
/// Returns an error if writing to the `writer` or JSON serialization fails.
pub fn print_sarif_with_root(
    writer: &mut impl Write,
    result: &AnalysisResult,
    root: Option<&std::path::Path>,
) -> std::io::Result<()> {
    let mut results = Vec::new();

    // Danger
    for f in &result.danger {
        results.push(make_sarif_result(
            &f.rule_id,
            &f.message,
            &f.file.to_string_lossy(),
            f.line,
            &f.severity,
            root,
        ));
    }
    // Secrets
    for s in &result.secrets {
        results.push(make_sarif_result(
            &s.rule_id,
            &s.message,
            &s.file.to_string_lossy(),
            s.line,
            "HIGH",
            root,
        ));
    }
    // Quality
    for q in &result.quality {
        results.push(make_sarif_result(
            &q.rule_id,
            &q.message,
            &q.file.to_string_lossy(),
            q.line,
            &q.severity,
            root,
        ));
    }
    // Taint
    for t in &result.taint_findings {
        results.push(make_sarif_result(
            &t.rule_id,
            &format!("{} (Source: {})", t.vuln_type, t.source),
            &t.file.to_string_lossy(),
            t.sink_line,
            &t.severity.to_string(),
            root,
        ));
    }
    // Unused
    add_unused_results(&mut results, result, root);

    // Parse Errors
    for e in &result.parse_errors {
        results.push(make_sarif_result(
            "parse-error",
            &format!("Parse error: {}", e.error),
            &e.file.to_string_lossy(),
            0,
            "HIGH",
            root,
        ));
    }

    let log = SarifLog {
        version: "2.1.0".to_owned(),
        schema: "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json".to_owned(),
        runs: vec![Run {
            tool: Tool {
                driver: Driver {
                    name: "CytoScnPy".to_owned(),
                    version: env!("CARGO_PKG_VERSION").to_owned(),
                    rules: Vec::new(),
                },
            },
            results,
        }],
    };

    serde_json::to_writer_pretty(writer, &log)?;
    Ok(())
}

fn add_unused_results(
    results: &mut Vec<SarifResult>,
    result: &AnalysisResult,
    root: Option<&std::path::Path>,
) {
    for f in &result.unused_functions {
        results.push(make_sarif_result(
            "unused-function",
            &format!("Unused function: {}", f.name),
            &f.file.to_string_lossy(),
            f.line,
            "LOW",
            root,
        ));
    }
    for c in &result.unused_classes {
        results.push(make_sarif_result(
            "unused-class",
            &format!("Unused class: {}", c.name),
            &c.file.to_string_lossy(),
            c.line,
            "LOW",
            root,
        ));
    }
    for i in &result.unused_imports {
        results.push(make_sarif_result(
            "unused-import",
            &format!("Unused import: {}", i.name),
            &i.file.to_string_lossy(),
            i.line,
            "LOW",
            root,
        ));
    }
    for v in &result.unused_variables {
        results.push(make_sarif_result(
            "unused-variable",
            &format!("Unused variable: {}", v.name),
            &v.file.to_string_lossy(),
            v.line,
            "LOW",
            root,
        ));
    }
    for m in &result.unused_methods {
        results.push(make_sarif_result(
            "unused-method",
            &format!("Unused method: {}", m.name),
            &m.file.to_string_lossy(),
            m.line,
            "LOW",
            root,
        ));
    }
    for p in &result.unused_parameters {
        results.push(make_sarif_result(
            "unused-parameter",
            &format!("Unused parameter: {}", p.name),
            &p.file.to_string_lossy(),
            p.line,
            "LOW",
            root,
        ));
    }
}

fn make_sarif_result(
    id: &str,
    msg: &str,
    file: &str,
    line: usize,
    severity: &str,
    root: Option<&std::path::Path>,
) -> SarifResult {
    let level = match severity.to_uppercase().as_str() {
        "CRITICAL" | "HIGH" => "error",
        "MEDIUM" => "warning",
        _ => "note",
    };

    let region = if line > 0 {
        Some(Region { start_line: line })
    } else {
        None
    };

    // Normalize path to URI
    let path = std::path::Path::new(file);
    let normalized_path = if let Some(r) = root {
        if r.as_os_str() == "." || r.as_os_str().is_empty() {
            path
        } else {
            path.strip_prefix(r).unwrap_or(path)
        }
    } else {
        path
    };

    let s = normalized_path.to_string_lossy().replace('\\', "/");
    let uri = s.strip_prefix("./").unwrap_or(&s).to_owned();

    SarifResult {
        rule_id: id.to_owned(),
        level: level.into(),
        message: Message {
            text: format!("{} ({}:{})", msg, uri, line),
        },
        locations: vec![Location {
            physical_location: PhysicalLocation {
                artifact_location: ArtifactLocation { uri },
                region,
            },
        }],
    }
}
