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
    let custom_sanitizers = analyzer
        .config
        .cytoscnpy
        .danger_config
        .custom_sanitizers
        .clone()
        .unwrap_or_default();
    let taint_analyzer =
        TaintAwareDangerAnalyzer::with_custom(custom_sources, custom_sinks, custom_sanitizers);
    let taint_context = taint_analyzer.build_taint_context(source, &file_path.to_path_buf());

    let mut filtered = TaintAwareDangerAnalyzer::filter_findings_with_taint(danger, &taint_context);
    TaintAwareDangerAnalyzer::enhance_severity_with_taint(&mut filtered, &taint_context);
    filtered
}

pub(super) fn apply_danger_config_filters(
    analyzer: &CytoScnPy,
    source: &str,
    danger: &mut Vec<Finding>,
) {
    apply_mitigation_filters(source, danger);

    if let Some(excluded) = &analyzer.config.cytoscnpy.danger_config.excluded_rules {
        danger.retain(|finding| !excluded.contains(&finding.rule_id));
    }

    if let Some(threshold) = &analyzer.config.cytoscnpy.danger_config.severity_threshold {
        let threshold_value = severity_value(threshold);
        danger.retain(|finding| severity_value(&finding.severity) >= threshold_value);
    }
}

fn apply_mitigation_filters(source: &str, danger: &mut Vec<Finding>) {
    let lines: Vec<&str> = source.lines().collect();
    danger.retain(|finding| !is_mitigated_finding(&lines, finding));
}

fn is_mitigated_finding(lines: &[&str], finding: &Finding) -> bool {
    const MITIGATION_AWARE_RULES: &[&str] = &[
        "CSP-D003", "CSP-D101", "CSP-D102", "CSP-D402", "CSP-D410", "CSP-D501", "CSP-D801",
    ];

    if !MITIGATION_AWARE_RULES.contains(&finding.rule_id.as_str()) || finding.line == 0 {
        return false;
    }

    let line_index = finding.line.saturating_sub(1);
    let Some(line_text) = lines.get(line_index) else {
        return false;
    };
    let line_lower = line_text.to_ascii_lowercase();
    let surrounding = surrounding_window(lines, line_index, 6);

    let has_trusted_marker = has_trusted_marker(&line_lower) || has_trusted_marker(&surrounding);
    if !has_trusted_marker {
        return false;
    }

    if matches!(
        finding.rule_id.as_str(),
        "CSP-D402" | "CSP-D410" | "CSP-D801"
    ) {
        return has_url_validation_evidence(&surrounding) || has_strong_url_name_hint(&line_lower);
    }

    true
}

fn surrounding_window(lines: &[&str], line_index: usize, window_size: usize) -> String {
    let start = line_index.saturating_sub(window_size);
    let end = (line_index + 1).min(lines.len());
    lines[start..end].join("\n").to_ascii_lowercase()
}

fn has_trusted_marker(text: &str) -> bool {
    [
        "validated_",
        "sanitized_",
        "trusted_",
        "safe_",
        "clean_",
        "allowlisted_",
        "whitelisted_",
        "checked_",
        "verified_",
    ]
    .iter()
    .any(|marker| contains_identifier_prefix(text, marker))
        || text.contains("validate(")
        || text.contains("sanitize(")
        || text.contains("allowlist(")
        || text.contains("whitelist(")
}

fn has_strong_url_name_hint(line_lower: &str) -> bool {
    (line_lower.contains("requests.")
        || line_lower.contains("httpx.")
        || line_lower.contains("urlopen")
        || line_lower.contains("redirect("))
        && (line_lower.contains("validated_url")
            || line_lower.contains("safe_url")
            || line_lower.contains("trusted_url")
            || line_lower.contains("allowlisted_url"))
}

fn has_url_validation_evidence(text: &str) -> bool {
    let has_url_parse = text.contains("urlparse(") || text.contains("urlsplit(");
    let has_scheme_check = text.contains(".scheme")
        && text.contains("http")
        && text.contains("https")
        && (text.contains("not in") || text.contains("in ("));
    let has_host_allowlist = text.contains("allowed_domains")
        || text.contains("allowed_hosts")
        || text.contains("trusted_domains")
        || text.contains("trusted_hosts")
        || text.contains("allowlist(")
        || text.contains("whitelist(");
    let has_private_ip_block = text.contains("ipaddress.ip_address")
        && (text.contains("is_private")
            || text.contains("is_loopback")
            || text.contains("is_link_local"));

    has_url_parse && (has_scheme_check || has_host_allowlist || has_private_ip_block)
}

fn contains_identifier_prefix(text: &str, prefix: &str) -> bool {
    let mut offset = 0;
    while let Some(index) = text[offset..].find(prefix) {
        let absolute = offset + index;
        let before = text[..absolute].chars().next_back();
        if before.is_some_and(is_identifier_char) {
            offset = absolute + prefix.len();
            continue;
        }
        return true;
    }
    false
}

fn is_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
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
