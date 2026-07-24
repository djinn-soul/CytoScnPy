use crate::analyzer::CytoScnPy;
use crate::rules::Finding;
use std::path::Path;

pub(super) fn module_name_from_path(file_path: &Path, root_path: &Path) -> String {
    let package_relative = package_relative_path(file_path);
    let relative_path = package_relative
        .as_deref()
        .unwrap_or_else(|| file_path.strip_prefix(root_path).unwrap_or(file_path));
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

/// Returns the path from the import root when the file belongs to a regular package.
/// This intentionally walks above the analysis root so scanning a package subdirectory
/// still produces the same qualified names as Python imports.
fn package_relative_path(file_path: &Path) -> Option<std::path::PathBuf> {
    let mut current = file_path.parent()?;
    let mut top_package = None;

    while current.join("__init__.py").is_file() {
        top_package = Some(current);
        current = current.parent()?;
    }

    let top_package = top_package?;
    file_path
        .strip_prefix(top_package.parent()?)
        .ok()
        .map(Path::to_path_buf)
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
    let sanitizers = crate::taint::sanitizers::SanitizerConfig::from_danger_config(
        &analyzer.config.cytoscnpy.danger_config,
    );
    let taint_analyzer =
        TaintAwareDangerAnalyzer::with_custom(custom_sources, custom_sinks, sanitizers);
    let taint_context = taint_analyzer.build_taint_context(source, &file_path.to_path_buf());

    let mut taint_sensitive_rules = crate::constants::get_taint_sensitive_rules().to_vec();
    let sanitizer_config = &analyzer.config.cytoscnpy.danger_config.sanitizers;
    if crate::taint::sanitizers::has_builtin_command_sanitizer(source)
        || has_configured_sanitizer(&sanitizer_config.command_injection)
        || analyzer
            .config
            .cytoscnpy
            .danger_config
            .custom_sanitizers
            .as_ref()
            .is_some_and(|items| !items.is_empty())
    {
        taint_sensitive_rules.push(crate::rules::ids::RULE_ID_SUBPROCESS);
    }
    if has_configured_sanitizer(&sanitizer_config.ssrf) {
        taint_sensitive_rules.push(crate::rules::ids::RULE_ID_URL_OPEN);
    }

    let mut filtered = TaintAwareDangerAnalyzer::filter_findings_with_taint_rules(
        danger,
        &taint_context,
        &taint_sensitive_rules,
    );
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
    const URL_RULES: &[&str] = &["CSP-D402", "CSP-D410", "CSP-D801"];
    if !URL_RULES.contains(&finding.rule_id.as_str()) || finding.line == 0 {
        return false;
    }

    let line_index = finding.line.saturating_sub(1);
    if lines.get(line_index).is_none() {
        return false;
    }
    let surrounding = surrounding_window(lines, line_index, 6);
    has_url_validation_evidence(&surrounding)
}

fn surrounding_window(lines: &[&str], line_index: usize, window_size: usize) -> String {
    let start = line_index.saturating_sub(window_size);
    let end = (line_index + 1).min(lines.len());
    lines[start..end].join("\n").to_ascii_lowercase()
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

    has_url_parse && has_scheme_check && (has_host_allowlist || has_private_ip_block)
}

fn has_configured_sanitizer(group: &crate::config::SanitizerGroup) -> bool {
    !group.return_value.is_empty() || !group.guard.is_empty() || !group.side_effect.is_empty()
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
