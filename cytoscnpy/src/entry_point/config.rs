use crate::cli::Cli;

pub(crate) struct AppConfig {
    pub(crate) config: crate::config::Config,
    pub(crate) exclude_folders: Vec<String>,
    pub(crate) include_folders: Vec<String>,
    pub(crate) include_tests: bool,
}

/// Loads project configuration and merges it with CLI flags.
pub(crate) fn setup_configuration(effective_paths: &[std::path::PathBuf], cli: &Cli) -> AppConfig {
    let config_path = effective_paths
        .first()
        .map_or(std::path::Path::new("."), std::path::PathBuf::as_path);
    let mut config = crate::config::Config::load_from_path(config_path);

    // CLI rule thresholds override config so analysis and gates stay consistent.
    if let Some(value) = cli.max_complexity {
        config.cytoscnpy.max_complexity = Some(value);
    }
    if let Some(value) = cli.max_nesting {
        config.cytoscnpy.max_nesting = Some(value);
    }
    if let Some(value) = cli.max_args {
        config.cytoscnpy.max_args = Some(value);
    }
    if let Some(value) = cli.max_lines {
        config.cytoscnpy.max_lines = Some(value);
    }
    if let Some(value) = cli.min_mi {
        config.cytoscnpy.min_mi = Some(value);
    }
    if let Some(value) = cli.fail_threshold {
        config.cytoscnpy.fail_threshold = Some(value);
    }

    let mut exclude_folders = config.cytoscnpy.exclude_folders.clone().unwrap_or_default();
    exclude_folders.extend(cli.exclude_folders.clone());

    let include_tests =
        cli.include.include_tests || config.cytoscnpy.include_tests.unwrap_or(false);

    let mut include_folders = config.cytoscnpy.include_folders.clone().unwrap_or_default();
    include_folders.extend(cli.include_folders.clone());

    AppConfig {
        config,
        exclude_folders,
        include_folders,
        include_tests,
    }
}

pub(crate) fn is_vscode_client(cli: &Cli) -> bool {
    matches!(cli.client, Some(crate::cli::ClientKind::Vscode))
}

pub(crate) fn resolve_scan_flag(
    cli_flag: bool,
    config_flag: Option<bool>,
    is_vscode: bool,
) -> bool {
    if is_vscode {
        cli_flag
    } else {
        // Default to true if not specified in CLI or config
        cli_flag || config_flag.unwrap_or(true)
    }
}
