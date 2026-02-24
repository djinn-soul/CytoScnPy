use super::pipeline::{run_pipeline, PipelineMode};
use crate::analyzer::{AnalysisResult, AnalysisSummary, CytoScnPy};
use crate::utils::LineIndex;

impl CytoScnPy {
    /// Analyzes a single string of code (mostly for testing).
    #[must_use]
    pub fn analyze_code(&self, code: &str, file_path: &std::path::Path) -> AnalysisResult {
        let source = code.to_owned();
        let line_index = LineIndex::new(&source);
        let ignored_lines = crate::utils::get_ignored_lines(&source);
        let is_test_file = crate::utils::is_test_path(&file_path.to_string_lossy());
        let module_name = file_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let output = run_pipeline(
            self,
            &source,
            file_path,
            &module_name,
            &line_index,
            &ignored_lines,
            is_test_file,
            PipelineMode::AnalyzeCode,
        );

        let total_definitions = output.visitor.definitions.len();
        let mut unused_functions = Vec::new();
        let mut unused_methods = Vec::new();
        let mut unused_classes = Vec::new();
        let mut unused_imports = Vec::new();
        let mut unused_variables = Vec::new();
        let mut unused_parameters = Vec::new();

        for def in output.visitor.definitions {
            if def.confidence >= self.confidence_threshold && def.references == 0 {
                if crate::utils::is_line_suppressed(&ignored_lines, def.line, "CSP-V001") {
                    continue;
                }

                match def.def_type.as_str() {
                    "function" => unused_functions.push(def),
                    "method" => unused_methods.push(def),
                    "class" => unused_classes.push(def),
                    "import" => unused_imports.push(def),
                    "variable" => unused_variables.push(def),
                    "parameter" => unused_parameters.push(def),
                    _ => {}
                }
            }
        }

        AnalysisResult {
            unused_functions,
            unused_methods,
            unused_imports,
            unused_classes,
            unused_variables,
            unused_parameters,
            secrets: output.secrets,
            danger: output.danger,
            quality: output.quality,
            taint_findings: Vec::new(),
            parse_errors: output.parse_errors,
            clones: Vec::new(),
            analysis_summary: AnalysisSummary {
                total_files: 1,
                total_lines_analyzed: source.lines().count(),
                total_definitions,
                ..AnalysisSummary::default()
            },
            file_metrics: Vec::new(),
        }
    }
}
