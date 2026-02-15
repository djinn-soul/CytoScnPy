use super::common::{apply_danger_config_filters, apply_taint_filters, split_lint_finding};
use super::pipeline::PipelineOutput;
use crate::analyzer::{apply_heuristics, apply_penalties, CytoScnPy};
use crate::halstead::analyze_halstead;
use crate::metrics::mi_compute;
use crate::raw_metrics::analyze_raw;
use crate::rules::secrets::scan_secrets;
use crate::rules::Finding;
use crate::utils::{LineIndex, Suppression};
use rustc_hash::{FxHashMap, FxHashSet};
use std::path::Path;

use crate::analyzer::traversal::collect_docstring_lines;

#[allow(clippy::too_many_arguments, clippy::cast_precision_loss)]
pub(super) fn analyze_parsed_module(
    analyzer: &CytoScnPy,
    source: &str,
    file_path: &Path,
    module_name: &str,
    line_index: &LineIndex,
    ignored_lines: &FxHashMap<usize, Suppression>,
    is_test_file: bool,
    output: &mut PipelineOutput<'_>,
    module: &ruff_python_ast::ModModule,
) {
    let mut docstring_lines = FxHashSet::default();
    if analyzer.enable_secrets && analyzer.config.cytoscnpy.secrets_config.skip_docstrings {
        collect_docstring_lines(&module.body, line_index, &mut docstring_lines, 0);
    }

    if analyzer.enable_secrets {
        output.secrets = scan_secrets(
            source,
            &file_path.to_path_buf(),
            &analyzer.config.cytoscnpy.secrets_config,
            Some(&docstring_lines),
            is_test_file,
        );
    }

    output
        .call_graph
        .build_from_module(&module.body, module_name);
    let entry_point_calls = crate::entry_point::detect_entry_point_calls(&module.body);

    for stmt in &module.body {
        output.framework_visitor.visit_stmt(stmt);
        output.test_visitor.visit_stmt(stmt);
        output.visitor.visit_stmt(stmt);
    }

    for call_name in &entry_point_calls {
        output.visitor.add_ref(call_name.clone());
        if !module_name.is_empty() {
            output.visitor.add_ref(format!("{module_name}.{call_name}"));
        }
    }

    for framework_ref in &output.framework_visitor.framework_references {
        output.visitor.add_ref(framework_ref.clone());
        if !module_name.is_empty() {
            output
                .visitor
                .add_ref(format!("{module_name}.{framework_ref}"));
        }
    }

    output.fixture_metadata = crate::analyzer::fixtures::collect_file_fixture_metadata(
        &output.visitor,
        &output.test_visitor,
        file_path,
        module_name,
    );
    let fixture_increments = crate::analyzer::fixtures::resolve_fixture_reference_increments(
        &output.fixture_metadata.fixture_definitions,
        &output.fixture_metadata.fixture_requests,
        &output.fixture_metadata.fixture_imports,
        &output.fixture_metadata.pytest_plugins,
    );
    for (full_name, count) in fixture_increments {
        *output.visitor.references.entry(full_name).or_insert(0) += count;
    }

    for export_name in output.visitor.exports.clone() {
        output.visitor.add_ref(export_name.clone());
        if !module_name.is_empty() {
            output
                .visitor
                .add_ref(format!("{module_name}.{export_name}"));
        }
    }

    CytoScnPy::sync_definition_references(
        &mut output.visitor.definitions,
        &output.visitor.references,
    );
    CytoScnPy::mark_captured_definitions(
        &mut output.visitor.definitions,
        &output.visitor.captured_definitions,
    );

    #[cfg(feature = "cfg")]
    CytoScnPy::refine_flow_sensitive(
        source,
        &mut output.visitor.definitions,
        &output.visitor.dynamic_scopes,
    );

    for def in &mut output.visitor.definitions {
        apply_penalties(
            def,
            &output.framework_visitor,
            &output.test_visitor,
            ignored_lines,
            analyzer.include_tests,
            &output.visitor.dynamic_scopes,
            module_name,
        );
        apply_heuristics(def);

        if def.def_type == "method"
            && output
                .visitor
                .self_referential_methods
                .contains(&def.full_name)
        {
            def.is_self_referential = true;
        }
    }

    let mut rules = Vec::new();
    if analyzer.enable_danger {
        rules.extend(crate::rules::danger::get_danger_rules());
    }
    if analyzer.enable_quality {
        rules.extend(crate::rules::quality::get_quality_rules(&analyzer.config));
    }

    if !rules.is_empty() {
        let mut linter = crate::linter::LinterVisitor::new(
            rules,
            file_path.to_path_buf(),
            line_index.clone(),
            analyzer.config.clone(),
            is_test_file,
        );
        for stmt in &module.body {
            linter.visit_stmt(stmt);
        }

        for finding in linter.findings {
            if crate::utils::is_line_suppressed(ignored_lines, finding.line, &finding.rule_id) {
                continue;
            }
            split_lint_finding(finding, &mut output.danger, &mut output.quality);
        }

        let filtered = apply_taint_filters(
            analyzer,
            source,
            file_path,
            std::mem::take(&mut output.danger),
        );
        output.danger = filtered;
        apply_danger_config_filters(analyzer, &mut output.danger);
    }

    if analyzer.enable_quality {
        let raw = analyze_raw(source);
        let halstead = analyze_halstead(&ruff_python_ast::Mod::Module(module.clone()));
        let complexity = crate::complexity::calculate_module_complexity(source).unwrap_or(1);
        output.file_complexity = complexity as f64;
        output.file_mi = mi_compute(halstead.volume, complexity, raw.sloc, raw.comments);

        if let Some(min_mi) = analyzer.config.cytoscnpy.min_mi {
            if output.file_mi < min_mi {
                output.quality.push(Finding {
                    message: format!(
                        "Maintainability Index too low ({:.2} < {:.2})",
                        output.file_mi, min_mi
                    ),
                    rule_id: crate::rules::ids::RULE_ID_MIN_MI.to_owned(),
                    category: "Maintainability".to_owned(),
                    file: file_path.to_path_buf(),
                    line: 1,
                    col: 0,
                    severity: "HIGH".to_owned(),
                });
            }
        }
    }
}
