use super::common::{apply_danger_config_filters, apply_taint_filters, split_lint_finding};
use super::pipeline::PipelineOutput;
use crate::analyzer::CytoScnPy;
use crate::halstead::analyze_halstead;
use crate::metrics::mi_compute;
use crate::raw_metrics::analyze_raw;
use crate::rules::Finding;
use crate::utils::{LineIndex, Suppression};
use rustc_hash::FxHashMap;
use std::path::Path;

pub(super) struct RuleEngineContext<'a> {
    pub(super) analyzer: &'a CytoScnPy,
    pub(super) source: &'a str,
    pub(super) module: &'a ruff_python_ast::ModModule,
    pub(super) file_path: &'a Path,
    pub(super) line_index: &'a LineIndex,
    pub(super) ignored_lines: &'a FxHashMap<usize, Suppression>,
    pub(super) is_test_file: bool,
}

pub(super) fn apply_rule_engine(ctx: &RuleEngineContext<'_>, output: &mut PipelineOutput<'_>) {
    apply_complexity_rules(ctx, output);
    apply_raw_rules(ctx, output);
}

pub(super) fn apply_complexity_rules(ctx: &RuleEngineContext<'_>, output: &mut PipelineOutput<'_>) {
    let mut rules = Vec::new();
    if ctx.analyzer.enable_danger {
        rules.extend(crate::rules::danger::get_danger_rules());
    }
    if ctx.analyzer.enable_quality {
        rules.extend(crate::rules::quality::get_quality_rules(
            &ctx.analyzer.config,
        ));
    }

    if rules.is_empty() {
        return;
    }

    let mut linter = crate::linter::LinterVisitor::new(
        rules,
        ctx.file_path.to_path_buf(),
        ctx.line_index.clone(),
        ctx.analyzer.config.clone(),
        ctx.is_test_file,
    );
    for stmt in &ctx.module.body {
        linter.visit_stmt(stmt);
    }

    for finding in linter.findings {
        if crate::utils::is_line_suppressed(ctx.ignored_lines, finding.line, &finding.rule_id)
            || ctx
                .analyzer
                .is_rule_ignored_for_path(ctx.file_path, &finding.rule_id)
        {
            continue;
        }
        split_lint_finding(finding, &mut output.danger, &mut output.quality);
    }

    let filtered = apply_taint_filters(
        ctx.analyzer,
        ctx.source,
        ctx.file_path,
        std::mem::take(&mut output.danger),
    );
    output.danger = filtered;
    apply_danger_config_filters(ctx.analyzer, &mut output.danger);
}

pub(super) fn apply_raw_rules(ctx: &RuleEngineContext<'_>, output: &mut PipelineOutput<'_>) {
    if !ctx.analyzer.enable_quality {
        return;
    }

    let raw = analyze_raw(ctx.source);
    let halstead = analyze_halstead(&ruff_python_ast::Mod::Module(ctx.module.clone()));
    let complexity = crate::complexity::calculate_module_complexity(ctx.source).unwrap_or(1);

    output.file_complexity = complexity as f64;
    output.file_mi = mi_compute(halstead.volume, complexity, raw.sloc, raw.comments);

    if let Some(min_mi) = ctx.analyzer.config.cytoscnpy.min_mi {
        if output.file_mi < min_mi {
            output.quality.push(Finding {
                message: format!(
                    "Maintainability Index too low ({:.2} < {:.2})",
                    output.file_mi, min_mi
                ),
                rule_id: crate::rules::ids::RULE_ID_MIN_MI.to_owned(),
                category: "Maintainability".to_owned(),
                file: ctx.file_path.to_path_buf(),
                line: 1,
                col: 0,
                severity: "HIGH".to_owned(),
            });
        }
    }
}
