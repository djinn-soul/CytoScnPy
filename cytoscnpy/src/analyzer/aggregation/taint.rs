use super::CytoScnPy;

pub(super) fn run_taint_analysis(
    analyzer: &CytoScnPy,
    files: &[std::path::PathBuf],
) -> Vec<crate::taint::types::TaintFinding> {
    if !analyzer.enable_danger
        || !analyzer
            .config
            .cytoscnpy
            .danger_config
            .enable_taint
            .unwrap_or(crate::constants::TAINT_ENABLED_DEFAULT)
    {
        return Vec::new();
    }

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
    let taint_config =
        crate::taint::analyzer::TaintConfig::with_custom(custom_sources, custom_sinks);
    let taint_analyzer = crate::taint::analyzer::TaintAnalyzer::new(taint_config);

    files
        .iter()
        .filter_map(|file_path| {
            let is_notebook = file_path.extension().is_some_and(|ext| ext == "ipynb");
            let source = if is_notebook {
                crate::ipynb::extract_notebook_code(file_path, Some(&analyzer.analysis_root)).ok()
            } else {
                std::fs::read_to_string(file_path).ok()
            };
            source.map(|content| (file_path.clone(), content))
        })
        .flat_map(|(path, source)| {
            let ignored = crate::utils::get_ignored_lines(&source);
            taint_analyzer
                .analyze_file(&source, &path)
                .into_iter()
                .filter(move |finding| {
                    !crate::utils::is_line_suppressed(&ignored, finding.sink_line, &finding.rule_id)
                        && !analyzer.is_rule_ignored_for_path(&finding.file, &finding.rule_id)
                })
                .collect::<Vec<_>>()
        })
        .collect()
}
