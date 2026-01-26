use crate::analyzer::AnalysisResult;
use crate::rules::Finding;
use std::io::Write;

/// Generates a `JUnit` XML report.
///
/// Schema:
/// <testsuites>
///   <testsuite name="CytoScnPy" tests="..." failures="..." errors="0" time="...">
///     <testcase name="..." classname="...">
///       <failure message="...">...</failure>
///     </testcase>
///   </testsuite>
/// </testsuites>
///
/// # Errors
///
/// Returns an error if writing to the `writer` fails.
pub fn print_junit(writer: &mut impl Write, result: &AnalysisResult) -> std::io::Result<()> {
    print_junit_with_root(writer, result, None)
}

/// Generates a `JUnit` XML report with an optional root path.
///
/// # Errors
///
/// Returns an error if writing to the `writer` fails.
pub fn print_junit_with_root(
    writer: &mut impl Write,
    result: &AnalysisResult,
    _root: Option<&std::path::Path>,
) -> std::io::Result<()> {
    writeln!(writer, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
    writeln!(writer, "<testsuites>")?;

    let total_findings = count_total_findings(result);

    // We treat every finding as a failure for now
    writeln!(
        writer,
        "  <testsuite name=\"CytoScnPy\" tests=\"{total_findings}\" failures=\"{total_findings}\" errors=\"0\">"
    )?;

    write_findings_to_junit(writer, result)?;

    writeln!(writer, "  </testsuite>")?;
    writeln!(writer, "</testsuites>")?;
    Ok(())
}

fn count_total_findings(result: &AnalysisResult) -> usize {
    result.danger.len()
        + result.secrets.len()
        + result.quality.len()
        + result.taint_findings.len()
        + result.unused_functions.len()
        + result.unused_parameters.len()
        + result.unused_classes.len()
        + result.unused_imports.len()
        + result.unused_variables.len()
        + result.unused_methods.len()
        + result.parse_errors.len()
}

fn write_findings_to_junit(
    writer: &mut impl Write,
    result: &AnalysisResult,
) -> std::io::Result<()> {
    // Security Findings
    for finding in &result.danger {
        write_testcase(writer, "Security", finding)?;
    }
    // Secrets
    for secret in &result.secrets {
        writeln!(
            writer,
            "    <testcase name=\"{}\" classname=\"{}\">",
            escape_xml(&secret.rule_id),
            escape_xml(&secret.file.to_string_lossy())
        )?;
        writeln!(
            writer,
            "      <failure message=\"{}\">Line {}: {}</failure>",
            escape_xml(&secret.message),
            secret.line,
            escape_xml(&secret.message)
        )?;
        writeln!(writer, "    </testcase>")?;
    }
    // Quality Findings
    for finding in &result.quality {
        write_testcase(writer, "Quality", finding)?;
    }
    // Taint Findings
    for finding in &result.taint_findings {
        let msg = format!("{} (Source: {})", finding.vuln_type, finding.source);
        writeln!(
            writer,
            "    <testcase name=\"{}\" classname=\"{}\">",
            escape_xml(&format!("Taint:{}", finding.rule_id)),
            escape_xml(&finding.file.to_string_lossy())
        )?;
        writeln!(
            writer,
            "      <failure message=\"{}\">Line {}: {} ({}:{})</failure>",
            escape_xml(&msg),
            finding.sink_line,
            escape_xml(&msg),
            escape_xml(&finding.file.to_string_lossy()),
            finding.sink_line
        )?;
        writeln!(writer, "    </testcase>")?;
    }

    // Unused Code
    write_unused_code_to_junit(writer, result)?;

    // Parse Errors
    for error in &result.parse_errors {
        writeln!(
            writer,
            "    <testcase name=\"ParseError\" classname=\"{}\">",
            escape_xml(&error.file.to_string_lossy())
        )?;
        writeln!(
            writer,
            "      <failure message=\"{}\">{}</failure>",
            escape_xml(&error.error),
            escape_xml(&error.error)
        )?;
        writeln!(writer, "    </testcase>")?;
    }
    Ok(())
}

fn write_unused_code_to_junit(
    writer: &mut impl Write,
    result: &AnalysisResult,
) -> std::io::Result<()> {
    for func in &result.unused_functions {
        write_unused(
            writer,
            "UnusedFunction",
            &func.name,
            &func.file.to_string_lossy(),
            func.line,
        )?;
    }
    for cls in &result.unused_classes {
        write_unused(
            writer,
            "UnusedClass",
            &cls.name,
            &cls.file.to_string_lossy(),
            cls.line,
        )?;
    }
    for imp in &result.unused_imports {
        write_unused(
            writer,
            "UnusedImport",
            &imp.name,
            &imp.file.to_string_lossy(),
            imp.line,
        )?;
    }
    for var in &result.unused_variables {
        write_unused(
            writer,
            "UnusedVariable",
            &var.name,
            &var.file.to_string_lossy(),
            var.line,
        )?;
    }
    for method in &result.unused_methods {
        write_unused(
            writer,
            "UnusedMethod",
            &method.name,
            &method.file.to_string_lossy(),
            method.line,
        )?;
    }
    for param in &result.unused_parameters {
        write_unused(
            writer,
            "UnusedParameter",
            &param.name,
            &param.file.to_string_lossy(),
            param.line,
        )?;
    }
    Ok(())
}

fn write_testcase(
    writer: &mut impl Write,
    category: &str,
    finding: &Finding,
) -> std::io::Result<()> {
    writeln!(
        writer,
        "    <testcase name=\"{}\" classname=\"{}\">",
        escape_xml(&format!("{category}:{}", finding.rule_id)),
        escape_xml(&finding.file.to_string_lossy())
    )?;
    writeln!(
        writer,
        "      <failure message=\"{}\">Line {}: {} ({}:{})</failure>",
        escape_xml(&finding.message),
        finding.line,
        escape_xml(&finding.message),
        escape_xml(&finding.file.to_string_lossy()),
        finding.line
    )?;
    writeln!(writer, "    </testcase>")?;
    Ok(())
}

fn write_unused(
    writer: &mut impl Write,
    rule_id: &str,
    name: &str,
    file: &str,
    line: usize,
) -> std::io::Result<()> {
    writeln!(
        writer,
        "    <testcase name=\"{}\" classname=\"{}\">",
        escape_xml(rule_id),
        escape_xml(file)
    )?;
    writeln!(
        writer,
        "      <failure message=\"Unused: {}\">Line {}: Unused {} '{}' ({}:{})</failure>",
        escape_xml(name),
        line,
        rule_id,
        escape_xml(name),
        escape_xml(file),
        line
    )?;
    writeln!(writer, "    </testcase>")?;
    Ok(())
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
