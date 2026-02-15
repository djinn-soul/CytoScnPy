use super::pipeline::PipelineOutput;
use super::rule_engine::{apply_rule_engine, RuleEngineContext};
use crate::analyzer::{apply_heuristics, apply_penalties, CytoScnPy};
use crate::rules::secrets::scan_secrets;
use crate::utils::{LineIndex, Suppression};
use rustc_hash::{FxHashMap, FxHashSet};
use std::path::Path;

use crate::analyzer::traversal::collect_docstring_lines;

pub(super) struct ParsedModuleContext<'a> {
    pub(super) analyzer: &'a CytoScnPy,
    pub(super) source: &'a str,
    pub(super) file_path: &'a Path,
    pub(super) module_name: &'a str,
    pub(super) line_index: &'a LineIndex,
    pub(super) ignored_lines: &'a FxHashMap<usize, Suppression>,
    pub(super) is_test_file: bool,
    pub(super) module: &'a ruff_python_ast::ModModule,
}

#[allow(clippy::cast_precision_loss)]
pub(super) fn analyze_parsed_module(
    ctx: &ParsedModuleContext<'_>,
    output: &mut PipelineOutput<'_>,
) {
    let mut docstring_lines = FxHashSet::default();
    if ctx.analyzer.enable_secrets && ctx.analyzer.config.cytoscnpy.secrets_config.skip_docstrings {
        collect_docstring_lines(&ctx.module.body, ctx.line_index, &mut docstring_lines, 0);
    }

    if ctx.analyzer.enable_secrets {
        output.secrets = scan_secrets(
            ctx.source,
            &ctx.file_path.to_path_buf(),
            &ctx.analyzer.config.cytoscnpy.secrets_config,
            Some(&docstring_lines),
            ctx.is_test_file,
        );
    }

    output
        .call_graph
        .build_from_module(&ctx.module.body, ctx.module_name);
    let entry_point_calls = crate::entry_point::detect_entry_point_calls(&ctx.module.body);

    for stmt in &ctx.module.body {
        output.framework_visitor.visit_stmt(stmt);
        output.test_visitor.visit_stmt(stmt);
        output.visitor.visit_stmt(stmt);
    }

    for call_name in &entry_point_calls {
        output.visitor.add_ref(call_name.clone());
        if !ctx.module_name.is_empty() {
            output
                .visitor
                .add_ref(format!("{}.{}", ctx.module_name, call_name));
        }
    }

    for framework_ref in &output.framework_visitor.framework_references {
        output.visitor.add_ref(framework_ref.clone());
        if !ctx.module_name.is_empty() {
            output
                .visitor
                .add_ref(format!("{}.{}", ctx.module_name, framework_ref));
        }
    }

    output.fixture_metadata = crate::analyzer::fixtures::collect_file_fixture_metadata(
        &output.visitor,
        &output.test_visitor,
        ctx.file_path,
        ctx.module_name,
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
        if !ctx.module_name.is_empty() {
            output
                .visitor
                .add_ref(format!("{}.{}", ctx.module_name, export_name));
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
        ctx.source,
        &mut output.visitor.definitions,
        &output.visitor.dynamic_scopes,
    );

    for def in &mut output.visitor.definitions {
        apply_penalties(
            def,
            &output.framework_visitor,
            &output.test_visitor,
            ctx.ignored_lines,
            ctx.analyzer.include_tests,
            &output.visitor.dynamic_scopes,
            ctx.module_name,
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

    apply_rule_engine(
        &RuleEngineContext {
            analyzer: ctx.analyzer,
            source: ctx.source,
            module: ctx.module,
            file_path: ctx.file_path,
            line_index: ctx.line_index,
            ignored_lines: ctx.ignored_lines,
            is_test_file: ctx.is_test_file,
        },
        output,
    );
}
