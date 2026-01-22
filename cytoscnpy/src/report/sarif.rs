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
pub fn print_sarif(writer: &mut impl Write, result: &AnalysisResult) -> std::io::Result<()> {
    print_sarif_with_root(writer, result, None)
}

/// Generates a SARIF (Static Analysis Results Interchange Format) report with an optional root path.
pub fn print_sarif_with_root(
    writer: &mut impl Write,
    result: &AnalysisResult,
    root: Option<&std::path::Path>,
) -> std::io::Result<()> {
    let mut results = Vec::new();
    let rules = Vec::new(); // Ideally we'd collect all unique rules here

    // Helper to map and add findings
    let mut add = |id: &str, msg: &str, file: &str, line: usize, severity: &str| {
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

        // Note: region is technically optional in SARIF, but highly recommended.
        // If line is 0 (GLOBAL/Parse Error), we omit region or could set to 1.
        // Omitting region is safer than lying about line 1 if it's truly global.

        // Normalize path to URI
        let path = std::path::Path::new(file);

        // First attempt to make path relative to root if absolute
        let normalized_path = if let Some(r) = root {
            if path.is_absolute() {
                path.strip_prefix(r).unwrap_or(path)
            } else {
                path
            }
        } else {
            path
        };

        let uri = if normalized_path.is_absolute() {
            let s = normalized_path.to_string_lossy().replace('\\', "/");
            if s.starts_with('/') {
                format!("file://{s}")
            } else {
                format!("file:///{s}")
            }
        } else {
            normalized_path.to_string_lossy().replace('\\', "/")
        };

        results.push(SarifResult {
            rule_id: id.to_string(),
            level: level.to_string(),
            message: Message {
                text: msg.to_string(),
            },
            locations: vec![Location {
                physical_location: PhysicalLocation {
                    artifact_location: ArtifactLocation { uri },
                    region,
                },
            }],
        });
    };

    // Danger
    for f in &result.danger {
        add(
            &f.rule_id,
            &f.message,
            &f.file.to_string_lossy(),
            f.line,
            &f.severity,
        );
    }
    // Secrets
    for s in &result.secrets {
        add(
            &s.rule_id,
            &s.message,
            &s.file.to_string_lossy(),
            s.line,
            "HIGH",
        );
    }
    // Quality
    for q in &result.quality {
        add(
            &q.rule_id,
            &q.message,
            &q.file.to_string_lossy(),
            q.line,
            &q.severity,
        );
    }
    // Taint
    for t in &result.taint_findings {
        let msg = format!("{} (Source: {})", t.vuln_type, t.source);
        add(
            &t.rule_id,
            &msg,
            &t.file.to_string_lossy(),
            t.sink_line,
            &t.severity.to_string(),
        );
    }
    // Unused
    for f in &result.unused_functions {
        add(
            "unused-function",
            &format!("Unused function: {}", f.name),
            &f.file.to_string_lossy(),
            f.line,
            "LOW",
        );
    }
    for c in &result.unused_classes {
        add(
            "unused-class",
            &format!("Unused class: {}", c.name),
            &c.file.to_string_lossy(),
            c.line,
            "LOW",
        );
    }
    for i in &result.unused_imports {
        add(
            "unused-import",
            &format!("Unused import: {}", i.name),
            &i.file.to_string_lossy(),
            i.line,
            "LOW",
        );
    }
    for v in &result.unused_variables {
        add(
            "unused-variable",
            &format!("Unused variable: {}", v.name),
            &v.file.to_string_lossy(),
            v.line,
            "LOW",
        );
    }
    for m in &result.unused_methods {
        add(
            "unused-method",
            &format!("Unused method: {}", m.name),
            &m.file.to_string_lossy(),
            m.line,
            "LOW",
        );
    }
    for p in &result.unused_parameters {
        add(
            "unused-parameter",
            &format!("Unused parameter: {}", p.name),
            &p.file.to_string_lossy(),
            p.line,
            "LOW",
        );
    }

    // Parse Errors
    for e in &result.parse_errors {
        add(
            "parse-error",
            &format!("Parse error: {}", e.error),
            &e.file.to_string_lossy(),
            0,
            "HIGH",
        );
    }

    let log = SarifLog {
        version: "2.1.0".to_owned(),
        schema: "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json".to_owned(),
        runs: vec![Run {
            tool: Tool {
                driver: Driver {
                    name: "CytoScnPy".to_owned(),
                    version: env!("CARGO_PKG_VERSION").to_owned(),
                    rules,
                },
            },
            results,
        }],
    };

    serde_json::to_writer_pretty(writer, &log)?;
    Ok(())
}
