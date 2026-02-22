//! Builder-style methods for CytoScnPy analyzer.

use globset::GlobBuilder;
use rustc_hash::{FxHashMap, FxHashSet};

use super::{CytoScnPy, PerFileIgnoreRule};
use crate::config::Config;

impl CytoScnPy {
    /// Creates a new `CytoScnPy` analyzer instance with the given configuration.
    #[must_use]
    #[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
    pub fn new(
        confidence_threshold: u8,
        enable_secrets: bool,
        enable_danger: bool,
        enable_quality: bool,
        include_tests: bool,
        exclude_folders: Vec<String>,
        include_folders: Vec<String>,
        include_ipynb: bool,
        ipynb_cells: bool,
        config: Config,
    ) -> Self {
        let per_file_ignore_rules =
            build_per_file_ignore_rules(config.cytoscnpy.per_file_ignores.as_ref());

        #[allow(deprecated)]
        Self {
            confidence_threshold,
            enable_secrets,
            enable_danger,
            enable_quality,
            include_tests,
            exclude_folders,
            include_folders,
            include_ipynb,
            ipynb_cells,
            total_files_analyzed: 0,
            total_lines_analyzed: 0,
            config,
            debug_delay_ms: None,
            progress_bar: None,
            verbose: false,
            analysis_root: std::path::PathBuf::from("."),
            whitelist_matcher: None,
            per_file_ignore_rules,
        }
    }

    /// Builder-style method to set the analysis root.
    #[must_use]
    pub fn with_root(mut self, root: std::path::PathBuf) -> Self {
        self.analysis_root = root;
        self
    }

    /// Builder-style method to set verbose mode.
    #[must_use]
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Builder-style method to set confidence threshold.
    #[must_use]
    pub fn with_confidence(mut self, threshold: u8) -> Self {
        self.confidence_threshold = threshold;
        self
    }

    /// Builder-style method to enable secrets scanning.
    #[must_use]
    pub fn with_secrets(mut self, enabled: bool) -> Self {
        self.enable_secrets = enabled;
        self
    }

    /// Builder-style method to enable danger (security) scanning.
    #[must_use]
    pub fn with_danger(mut self, enabled: bool) -> Self {
        self.enable_danger = enabled;
        self
    }

    /// Builder-style method to enable quality scanning.
    #[must_use]
    pub fn with_quality(mut self, enabled: bool) -> Self {
        self.enable_quality = enabled;
        self
    }

    /// Builder-style method to include test files.
    #[must_use]
    pub fn with_tests(mut self, include: bool) -> Self {
        self.include_tests = include;
        self
    }

    /// Builder-style method to set excluded folders.
    #[must_use]
    pub fn with_excludes(mut self, folders: Vec<String>) -> Self {
        self.exclude_folders = folders;
        self
    }

    /// Builder-style method to set included folders.
    #[must_use]
    pub fn with_includes(mut self, folders: Vec<String>) -> Self {
        self.include_folders = folders;
        self
    }

    /// Builder-style method to include `IPython` notebooks.
    #[must_use]
    pub fn with_ipynb(mut self, include: bool) -> Self {
        self.include_ipynb = include;
        self
    }

    /// Builder-style method to enable cell-level reporting.
    #[must_use]
    pub fn with_ipynb_cells(mut self, enabled: bool) -> Self {
        self.ipynb_cells = enabled;
        self
    }

    /// Builder-style method to set config.
    #[must_use]
    pub fn with_config(mut self, config: Config) -> Self {
        self.config = config;
        self.per_file_ignore_rules =
            build_per_file_ignore_rules(self.config.cytoscnpy.per_file_ignores.as_ref());
        self
    }

    /// Builder-style method to set debug delay.
    #[must_use]
    pub fn with_debug_delay(mut self, delay_ms: Option<u64>) -> Self {
        self.debug_delay_ms = delay_ms;
        self
    }

    /// Counts the total number of Python files that would be analyzed.
    /// Useful for setting up a progress bar before analysis.
    /// Respects .gitignore files in addition to hardcoded defaults.
    #[must_use]
    pub fn count_files(&self, paths: &[std::path::PathBuf]) -> usize {
        paths
            .iter()
            .map(|path| {
                crate::utils::collect_python_files_gitignore(
                    path,
                    &self.exclude_folders,
                    &self.include_folders,
                    self.include_ipynb,
                    self.verbose,
                )
                .0
                .len()
            })
            .sum()
    }
}

fn build_per_file_ignore_rules(
    per_file_ignores: Option<&FxHashMap<String, Vec<String>>>,
) -> Vec<PerFileIgnoreRule> {
    let mut rules = Vec::new();
    if let Some(mapping) = per_file_ignores {
        for (pattern, ids) in mapping {
            match GlobBuilder::new(pattern).literal_separator(true).build() {
                Ok(glob) => {
                    let rule_ids = ids
                        .iter()
                        .map(|id| id.trim().to_uppercase())
                        .filter(|id| !id.is_empty())
                        .collect::<FxHashSet<_>>();

                    if rule_ids.is_empty() {
                        continue;
                    }

                    rules.push(PerFileIgnoreRule {
                        matcher: glob.compile_matcher(),
                        rule_ids,
                    });
                }
                Err(err) => {
                    eprintln!("[WARN] Skipping invalid per-file ignore glob '{pattern}': {err}");
                }
            }
        }
    }
    rules
}
