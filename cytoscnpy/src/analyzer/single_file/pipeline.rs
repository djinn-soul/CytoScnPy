use super::duck_typing::apply_duck_typing_usage;
use super::parsed_module::analyze_parsed_module;
use crate::analyzer::{CytoScnPy, ParseError};
use crate::framework::FrameworkAwareVisitor;
use crate::rules::secrets::scan_secrets;
use crate::taint::call_graph::CallGraph;
use crate::test_utils::TestAwareVisitor;
use crate::utils::{LineIndex, Suppression};
use crate::visitor::CytoScnPyVisitor;
use rustc_hash::FxHashMap;
use std::path::Path;

use crate::analyzer::traversal::convert_byte_range_to_line;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PipelineMode {
    ProcessFile,
    AnalyzeCode,
}

pub(super) struct PipelineOutput<'a> {
    pub(super) visitor: CytoScnPyVisitor<'a>,
    pub(super) framework_visitor: FrameworkAwareVisitor<'a>,
    pub(super) test_visitor: TestAwareVisitor<'a>,
    pub(super) secrets: Vec<crate::rules::secrets::SecretFinding>,
    pub(super) danger: Vec<crate::rules::Finding>,
    pub(super) quality: Vec<crate::rules::Finding>,
    pub(super) parse_errors: Vec<ParseError>,
    pub(super) call_graph: CallGraph,
    pub(super) fixture_metadata: crate::analyzer::fixtures::FileFixtureMetadata,
    pub(super) file_complexity: f64,
    pub(super) file_mi: f64,
}

#[allow(clippy::too_many_arguments, clippy::cast_precision_loss)]
pub(super) fn run_pipeline<'a>(
    analyzer: &CytoScnPy,
    source: &str,
    file_path: &Path,
    module_name: &str,
    line_index: &'a LineIndex,
    ignored_lines: &FxHashMap<usize, Suppression>,
    is_test_file: bool,
    mode: PipelineMode,
) -> PipelineOutput<'a> {
    let visitor = CytoScnPyVisitor::with_project_type(
        file_path.to_path_buf(),
        module_name.to_owned(),
        line_index,
        analyzer.config.cytoscnpy.project_type.unwrap_or_default(),
    );
    let framework_visitor = FrameworkAwareVisitor::new(line_index);
    let test_visitor = TestAwareVisitor::new(file_path, line_index);

    let mut output = PipelineOutput {
        visitor,
        framework_visitor,
        test_visitor,
        secrets: Vec::new(),
        danger: Vec::new(),
        quality: Vec::new(),
        parse_errors: Vec::new(),
        call_graph: CallGraph::new(),
        fixture_metadata: crate::analyzer::fixtures::FileFixtureMetadata::default(),
        file_complexity: 0.0,
        file_mi: 0.0,
    };

    match ruff_python_parser::parse_module(source) {
        Ok(parsed) => {
            let module = parsed.into_syntax();
            analyze_parsed_module(
                analyzer,
                source,
                file_path,
                module_name,
                line_index,
                ignored_lines,
                is_test_file,
                &mut output,
                &module,
            );

            if matches!(mode, PipelineMode::AnalyzeCode) {
                apply_duck_typing_usage(
                    &mut output.visitor.definitions,
                    &output.visitor.protocol_methods,
                );
            }
        }
        Err(error) => {
            if analyzer.enable_secrets {
                output.secrets = scan_secrets(
                    source,
                    &file_path.to_path_buf(),
                    &analyzer.config.cytoscnpy.secrets_config,
                    None,
                    is_test_file,
                );
            }

            let mut message = format!("{error}");
            if matches!(mode, PipelineMode::ProcessFile) {
                message = convert_byte_range_to_line(&message, source);
            }

            output.parse_errors.push(ParseError {
                file: file_path.to_path_buf(),
                error: message,
            });
        }
    }

    output
}
