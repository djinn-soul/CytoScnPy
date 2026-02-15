use crate::analyzer::CytoScnPy;
use crate::rules::Finding;
use std::path::Path;

pub(super) fn module_name_from_path(file_path: &Path, root_path: &Path) -> String {
    let relative_path = file_path.strip_prefix(root_path).unwrap_or(file_path);
    let components: Vec<&str> = relative_path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect();

    let mut module_parts = Vec::new();
    for (index, part) in components.iter().enumerate() {
        if index == components.len() - 1 {
            if let Some(stem) = Path::new(part).file_stem() {
                let stem_text = stem.to_string_lossy();
                if stem_text != "__init__" {
                    module_parts.push(stem_text.to_string());
                }
            }
        } else {
            module_parts.push((*part).to_owned());
        }
    }

    module_parts.join(".")
}

pub(super) fn split_lint_finding(
    finding: Finding,
    danger: &mut Vec<Finding>,
    quality: &mut Vec<Finding>,
) {
    if finding.rule_id.starts_with("CSP-D") || finding.rule_id.starts_with("CSP-X") {
        danger.push(finding);
    } else if finding.rule_id.starts_with("CSP-Q")
        || finding.rule_id.starts_with("CSP-L")
        || finding.rule_id.starts_with("CSP-C")
        || finding.rule_id.starts_with("CSP-P")
    {
        quality.push(finding);
    }
}

pub(super) fn apply_taint_filters(
    analyzer: &CytoScnPy,
    source: &str,
    file_path: &Path,
    danger: Vec<Finding>,
) -> Vec<Finding> {
    if !analyzer.enable_danger
        || !analyzer
            .config
            .cytoscnpy
            .danger_config
            .enable_taint
            .unwrap_or(crate::constants::TAINT_ENABLED_DEFAULT)
    {
        return danger;
    }

    use crate::rules::danger::taint_aware::TaintAwareDangerAnalyzer;

    let custom_sources = analyzer
        .config
        .cytoscnpy
        .danger_config
        .custom_sources
        .clone()
        .unwrap_or_default();
    let custom_sinks = analyzer
        .config
        .cytoscnpy
        .danger_config
        .custom_sinks
        .clone()
        .unwrap_or_default();
    let taint_analyzer = TaintAwareDangerAnalyzer::with_custom(custom_sources, custom_sinks);
    let taint_context = taint_analyzer.build_taint_context(source, &file_path.to_path_buf());

    let mut filtered = TaintAwareDangerAnalyzer::filter_findings_with_taint(danger, &taint_context);
    TaintAwareDangerAnalyzer::enhance_severity_with_taint(&mut filtered, &taint_context);
    filtered
}

pub(super) fn apply_danger_config_filters(analyzer: &CytoScnPy, danger: &mut Vec<Finding>) {
    if let Some(excluded) = &analyzer.config.cytoscnpy.danger_config.excluded_rules {
        danger.retain(|finding| !excluded.contains(&finding.rule_id));
    }

    if let Some(threshold) = &analyzer.config.cytoscnpy.danger_config.severity_threshold {
        let threshold_value = severity_value(threshold);
        danger.retain(|finding| severity_value(&finding.severity) >= threshold_value);
    }
}

fn severity_value(label: &str) -> u8 {
    match label.to_uppercase().as_str() {
        "CRITICAL" => 4,
        "HIGH" => 3,
        "MEDIUM" => 2,
        "LOW" => 1,
        _ => 0,
    }
}
