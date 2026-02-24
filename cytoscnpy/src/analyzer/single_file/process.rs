use super::common::module_name_from_path;
use super::pipeline::{run_pipeline, PipelineMode};
use crate::analyzer::{CytoScnPy, FileAnalysisResult};
use crate::halstead::HalsteadMetrics;
use crate::raw_metrics::{analyze_raw, RawMetrics};
use crate::utils::LineIndex;
use std::fs;
use std::path::Path;

impl CytoScnPy {
    /// Processes a single file (from disk or notebook) and returns analysis results.
    /// Used by the directory traversal for high-performance scanning.
    #[must_use]
    pub fn process_single_file(&self, file_path: &Path, root_path: &Path) -> FileAnalysisResult {
        let is_notebook = file_path.extension().is_some_and(|ext| ext == "ipynb");

        if let Some(delay_ms) = self.debug_delay_ms {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        }

        let is_test_file = crate::utils::is_test_path(&file_path.to_string_lossy());
        let source = match read_source(file_path, is_notebook, &self.analysis_root) {
            Ok(source) => source,
            Err(error) => return FileAnalysisResult::error(file_path, error),
        };

        let file_size = source.len();
        let line_index = LineIndex::new(&source);
        let ignored_lines = crate::utils::get_ignored_lines(&source);
        let module_name = module_name_from_path(file_path, root_path);

        let output = run_pipeline(
            self,
            &source,
            file_path,
            &module_name,
            &line_index,
            &ignored_lines,
            is_test_file,
            PipelineMode::ProcessFile,
        );

        if let Some(progress_bar) = &self.progress_bar {
            progress_bar.inc(1);
        }

        FileAnalysisResult {
            definitions: output.visitor.definitions,
            references: output.visitor.references,
            import_bindings: output.visitor.import_bindings,
            protocol_methods: output.visitor.protocol_methods,
            secrets: output.secrets,
            danger: output.danger,
            quality: output.quality,
            parse_errors: output.parse_errors,
            line_count: source.lines().count(),
            raw_metrics: if self.enable_quality {
                analyze_raw(&source)
            } else {
                RawMetrics::default()
            },
            halstead_metrics: HalsteadMetrics::default(),
            complexity: output.file_complexity,
            mi: output.file_mi,
            file_size,
            call_graph: output.call_graph,
            dynamic_imports: output.visitor.dynamic_imports,
            fixture_definitions: output.fixture_metadata.fixture_definitions,
            fixture_requests: output.fixture_metadata.fixture_requests,
            fixture_imports: output.fixture_metadata.fixture_imports,
            pytest_plugins: output.fixture_metadata.pytest_plugins,
        }
    }
}

fn read_source(
    file_path: &Path,
    is_notebook: bool,
    analysis_root: &std::path::Path,
) -> Result<String, String> {
    if is_notebook {
        return crate::ipynb::extract_notebook_code(file_path, Some(analysis_root))
            .map_err(|error| format!("Failed to parse notebook: {error}"));
    }

    fs::read_to_string(file_path).map_err(|error| format!("Failed to read file: {error}"))
}
