#[allow(clippy::struct_excessive_bools)] // Flags are intentional and mirror CLI/config switches
pub(crate) struct AnalysisContext {
    pub(crate) include_tests: bool,
    pub(crate) include_ipynb: bool,
    pub(crate) confidence: u8,
    pub(crate) secrets: bool,
    pub(crate) danger: bool,
    pub(crate) quality: bool,
    pub(crate) exclude_folders: Vec<String>,
    pub(crate) include_folders: Vec<String>,
    pub(crate) is_structured: bool,
}

pub(crate) fn build_analysis_context(
    cli_var: &crate::cli::Cli,
    config: &crate::config::Config,
    is_vscode_client: bool,
    base_exclude_folders: &[String],
    base_include_folders: &[String],
) -> AnalysisContext {
    let include_tests =
        cli_var.include.include_tests || config.cytoscnpy.include_tests.unwrap_or(false);
    let include_ipynb =
        cli_var.include.include_ipynb || config.cytoscnpy.include_ipynb.unwrap_or(false);
    let confidence = cli_var
        .confidence
        .or(config.cytoscnpy.confidence)
        .unwrap_or(60);
    let secrets = crate::entry_point::config::resolve_scan_flag(
        cli_var.scan.secrets,
        config.cytoscnpy.secrets,
        is_vscode_client,
    );
    let danger = crate::entry_point::config::resolve_scan_flag(
        cli_var.scan.danger,
        config.cytoscnpy.danger,
        is_vscode_client,
    );

    // Auto-enable quality mode when:
    // - --quality flag is passed
    // - quality is enabled in config (except when `--client vscode`)
    // - --min-mi or --max-complexity thresholds are set
    // - --html flag is passed (for dashboard metrics)
    #[cfg(feature = "html_report")]
    let html_enabled = cli_var.output.html;
    #[cfg(not(feature = "html_report"))]
    let html_enabled = false;

    let quality = cli_var.scan.quality
        || (!is_vscode_client && config.cytoscnpy.quality.unwrap_or(false))
        || cli_var.min_mi.is_some()
        || cli_var.max_complexity.is_some()
        || (!is_vscode_client
            && (config.cytoscnpy.min_mi.is_some() || config.cytoscnpy.max_complexity.is_some()))
        || html_enabled;

    let is_structured = cli_var.output.json
        || matches!(
            cli_var.output.format,
            crate::cli::OutputFormat::Json
                | crate::cli::OutputFormat::Junit
                | crate::cli::OutputFormat::Sarif
                | crate::cli::OutputFormat::Gitlab
                | crate::cli::OutputFormat::Github
                | crate::cli::OutputFormat::Markdown
        );

    AnalysisContext {
        include_tests,
        include_ipynb,
        confidence,
        secrets,
        danger,
        quality,
        exclude_folders: base_exclude_folders.to_vec(),
        include_folders: base_include_folders.to_vec(),
        is_structured,
    }
}
