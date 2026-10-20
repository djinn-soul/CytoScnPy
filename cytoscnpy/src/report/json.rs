use std::io::Write;

use anyhow::Result;
use rayon::prelude::*;
use serde_json::Value;

use super::json_definition::SortedDefinition;
use super::json_render::{self, Section};
use super::stable_findings::stable_findings;

/// Serialize the complete analysis result with schema metadata and stable findings.
///
/// # Errors
///
/// Returns an error if the analysis result cannot be serialized as JSON.
pub fn machine_json_payload(result: &crate::analyzer::AnalysisResult) -> Result<String> {
    let mut payload = Vec::new();
    write_machine_json(result, &mut payload)?;
    Ok(String::from_utf8(payload)?)
}

/// Stream the payload of [`machine_json_payload`] to `writer`.
///
/// # Errors
///
/// Returns an error if the result cannot be serialized or written.
pub fn write_machine_json<W: Write>(
    result: &crate::analyzer::AnalysisResult,
    writer: &mut W,
) -> Result<()> {
    let stable = stable_findings(result);
    let schema_version = Value::String("2".to_owned());
    let mut sections = result_sections(result);
    sections.push(("schema_version", json_render::value(&schema_version)));
    sections.push(("stable_findings", json_render::sorted_keys(&stable)));

    let mut out = std::io::BufWriter::with_capacity(1 << 20, writer);
    json_render::write_object(&mut out, sections)?;
    out.flush()?;
    drop(out);
    stable.into_par_iter().for_each(drop);
    Ok(())
}

/// One section per serialized `AnalysisResult` field. The exhaustive
/// destructuring makes adding a result field a compile error until it is
/// listed here.
fn result_sections(result: &crate::analyzer::AnalysisResult) -> Vec<(&'static str, Section<'_>)> {
    use json_render::{array, value};
    let definitions = |items| json_render::sorted_view(items, SortedDefinition::from);
    let crate::analyzer::AnalysisResult {
        unused_functions,
        unused_methods,
        unused_imports,
        unused_classes,
        unused_variables,
        unused_parameters,
        unused_dependencies,
        missing_dependencies,
        missing_dependency_details,
        transitive_dependencies,
        dev_dependencies_in_production,
        stdlib_dependencies,
        secrets,
        danger,
        quality,
        taint_findings,
        parse_errors,
        clones,
        file_metrics,
        analysis_summary,
    } = result;

    let mut sections = vec![
        ("unused_functions", definitions(unused_functions)),
        ("unused_methods", definitions(unused_methods)),
        ("unused_imports", definitions(unused_imports)),
        ("unused_classes", definitions(unused_classes)),
        ("unused_variables", definitions(unused_variables)),
        ("unused_parameters", definitions(unused_parameters)),
        ("unused_dependencies", array(unused_dependencies)),
        ("missing_dependencies", array(missing_dependencies)),
        ("transitive_dependencies", array(transitive_dependencies)),
        (
            "dev_dependencies_in_production",
            array(dev_dependencies_in_production),
        ),
        ("stdlib_dependencies", array(stdlib_dependencies)),
        ("secrets", array(secrets)),
        ("danger", array(danger)),
        ("quality", array(quality)),
        ("taint_findings", array(taint_findings)),
        ("parse_errors", array(parse_errors)),
        ("clones", array(clones)),
        ("file_metrics", array(file_metrics)),
        ("analysis_summary", value(analysis_summary)),
    ];
    // Mirrors `skip_serializing_if = "Vec::is_empty"` on the field.
    if !missing_dependency_details.is_empty() {
        sections.push((
            "missing_dependency_details",
            array(missing_dependency_details),
        ));
    }
    sections
}
