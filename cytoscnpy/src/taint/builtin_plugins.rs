//! Built-in framework taint source and sink plugins.

use super::{TaintSinkPlugin, TaintSourcePlugin};
use crate::taint::sinks::check_sink as check_builtin_sink;
use crate::taint::sources::check_taint_source;
use crate::taint::types::{SinkMatch, TaintInfo, TaintSource};
use crate::utils::LineIndex;
use ruff_python_ast::{Expr, ExprCall};

/// Built-in Flask source plugin.
pub struct FlaskSourcePlugin;

impl TaintSourcePlugin for FlaskSourcePlugin {
    fn name(&self) -> &'static str {
        "Flask"
    }

    fn check_source(&self, expr: &Expr, line_index: &LineIndex) -> Option<TaintInfo> {
        check_taint_source(expr, line_index)
            .filter(|info| matches!(info.source, TaintSource::FlaskRequest(_)))
    }

    fn patterns(&self) -> Vec<String> {
        vec![
            "request.args".to_owned(),
            "request.form".to_owned(),
            "request.data".to_owned(),
            "request.json".to_owned(),
            "request.cookies".to_owned(),
            "request.files".to_owned(),
        ]
    }
}

/// Built-in Django source plugin.
pub struct DjangoSourcePlugin;

impl TaintSourcePlugin for DjangoSourcePlugin {
    fn name(&self) -> &'static str {
        "Django"
    }

    fn check_source(&self, expr: &Expr, line_index: &LineIndex) -> Option<TaintInfo> {
        check_taint_source(expr, line_index)
            .filter(|info| matches!(info.source, TaintSource::DjangoRequest(_)))
    }

    fn patterns(&self) -> Vec<String> {
        vec![
            "request.GET".to_owned(),
            "request.POST".to_owned(),
            "request.body".to_owned(),
            "request.COOKIES".to_owned(),
        ]
    }
}

/// Built-in input/environment source plugin.
pub struct BuiltinSourcePlugin;

impl TaintSourcePlugin for BuiltinSourcePlugin {
    fn name(&self) -> &'static str {
        "Builtin"
    }

    fn check_source(&self, expr: &Expr, line_index: &LineIndex) -> Option<TaintInfo> {
        check_taint_source(expr, line_index).filter(|info| {
            matches!(
                info.source,
                TaintSource::Input
                    | TaintSource::Environment
                    | TaintSource::CommandLine
                    | TaintSource::FileRead
                    | TaintSource::ExternalData
            )
        })
    }

    fn patterns(&self) -> Vec<String> {
        vec![
            "input()".to_owned(),
            "sys.argv".to_owned(),
            "os.environ".to_owned(),
        ]
    }
}

/// Azure Functions source plugin.
pub struct AzureSourcePlugin;

impl TaintSourcePlugin for AzureSourcePlugin {
    fn name(&self) -> &'static str {
        "AzureFunctions"
    }

    fn check_source(&self, expr: &Expr, line_index: &LineIndex) -> Option<TaintInfo> {
        check_taint_source(expr, line_index)
            .filter(|info| matches!(info.source, TaintSource::AzureFunctionsRequest(_)))
    }

    fn patterns(&self) -> Vec<String> {
        vec![
            "req.params".to_owned(),
            "req.route_params".to_owned(),
            "req.headers".to_owned(),
            "req.form".to_owned(),
            "req.get_json".to_owned(),
            "req.get_body".to_owned(),
        ]
    }
}

/// Built-in sink plugin.
pub struct BuiltinSinkPlugin;

impl TaintSinkPlugin for BuiltinSinkPlugin {
    fn name(&self) -> &'static str {
        "Builtin"
    }

    fn check_sink(&self, call: &ExprCall) -> Option<SinkMatch> {
        check_builtin_sink(call).map(|info| SinkMatch {
            name: info.name,
            rule_id: info.rule_id,
            vuln_type: info.vuln_type,
            severity: info.severity,
            dangerous_args: info.dangerous_args,
            dangerous_keywords: info.dangerous_keywords,
            remediation: info.remediation,
        })
    }

    fn patterns(&self) -> Vec<String> {
        crate::taint::sinks::SINK_PATTERNS
            .iter()
            .map(|pattern| (*pattern).to_owned())
            .collect()
    }
}
