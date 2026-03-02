use crate::rules::danger::utils::get_call_name;
use crate::taint::sinks::check_sink as check_builtin_sink;
use crate::taint::sources::check_taint_source;
use crate::taint::types::{Severity, SinkMatch, TaintInfo, TaintSource, VulnType};
use crate::utils::LineIndex;
use ruff_python_ast::{Expr, ExprCall};
use ruff_text_size::Ranged;
use std::sync::Arc;

/// Trait for custom taint source plugins.
pub trait TaintSourcePlugin: Send + Sync {
    /// Returns the name of this source plugin.
    fn name(&self) -> &str;

    /// Checks if an expression is a taint source.
    fn check_source(&self, expr: &Expr, line_index: &LineIndex) -> Option<TaintInfo>;

    /// Returns the source patterns this plugin handles (for documentation).
    fn patterns(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Trait for custom taint sink plugins.
pub trait TaintSinkPlugin: Send + Sync {
    /// Returns the name of this sink plugin.
    fn name(&self) -> &str;

    /// Checks if a call expression is a dangerous sink.
    fn check_sink(&self, call: &ExprCall) -> Option<SinkMatch>;

    /// Returns the sink patterns this plugin handles.
    fn patterns(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Trait for custom sanitizer plugins.
pub trait SanitizerPlugin: Send + Sync {
    /// Returns the name of this sanitizer plugin.
    fn name(&self) -> &str;

    /// Checks if a call sanitizes taint.
    fn is_sanitizer(&self, call: &ExprCall) -> bool;

    /// Returns which vulnerability types this sanitizer addresses.
    fn sanitizes_vuln_types(&self) -> Vec<VulnType> {
        Vec::new()
    }
}

/// Registry for taint analysis plugins.
#[derive(Default)]
pub struct PluginRegistry {
    /// Registered source plugins.
    pub sources: Vec<Arc<dyn TaintSourcePlugin>>,
    /// Registered sink plugins.
    pub sinks: Vec<Arc<dyn TaintSinkPlugin>>,
    /// Registered sanitizer plugins.
    pub sanitizers: Vec<Arc<dyn SanitizerPlugin>>,
}

impl PluginRegistry {
    /// Creates a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a source plugin.
    pub fn register_source<T: TaintSourcePlugin + 'static>(&mut self, plugin: T) {
        self.sources.push(Arc::new(plugin));
    }

    /// Registers a sink plugin.
    pub fn register_sink<T: TaintSinkPlugin + 'static>(&mut self, plugin: T) {
        self.sinks.push(Arc::new(plugin));
    }

    /// Registers a sanitizer plugin.
    pub fn register_sanitizer<T: SanitizerPlugin + 'static>(&mut self, plugin: T) {
        self.sanitizers.push(Arc::new(plugin));
    }

    /// Checks all source plugins for a match.
    pub fn check_sources(&self, expr: &Expr, line_index: &LineIndex) -> Option<TaintInfo> {
        for plugin in &self.sources {
            if let Some(info) = plugin.check_source(expr, line_index) {
                return Some(info);
            }
        }
        None
    }

    /// Checks all sink plugins for a match.
    pub fn check_sinks(&self, call: &ExprCall) -> Option<SinkMatch> {
        for plugin in &self.sinks {
            if let Some(sink) = plugin.check_sink(call) {
                return Some(sink);
            }
        }
        None
    }

    /// Checks if any sanitizer plugin matches.
    pub fn is_sanitizer(&self, call: &ExprCall) -> bool {
        for plugin in &self.sanitizers {
            if plugin.is_sanitizer(call) {
                return true;
            }
        }
        false
    }
}

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

/// Plugin for dynamic patterns from configuration.
pub struct DynamicPatternPlugin {
    /// List of custom source patterns to match.
    pub sources: Vec<String>,
    /// List of custom sink patterns to match.
    pub sinks: Vec<String>,
}

impl TaintSourcePlugin for DynamicPatternPlugin {
    fn name(&self) -> &'static str {
        "DynamicConfig"
    }

    fn check_source(&self, expr: &Expr, line_index: &LineIndex) -> Option<TaintInfo> {
        let target = if let Expr::Call(call) = expr {
            &call.func
        } else {
            expr
        };

        if let Some(call_name) = get_call_name(target) {
            for pattern in &self.sources {
                if &call_name == pattern {
                    return Some(TaintInfo::new(
                        TaintSource::Custom(pattern.clone()),
                        line_index.line_index(expr.range().start()),
                    ));
                }
            }
        }

        None
    }

    fn patterns(&self) -> Vec<String> {
        self.sources.clone()
    }
}

impl TaintSinkPlugin for DynamicPatternPlugin {
    fn name(&self) -> &'static str {
        "DynamicConfig"
    }

    fn check_sink(&self, call: &ExprCall) -> Option<SinkMatch> {
        if let Some(call_name) = get_call_name(&call.func) {
            for pattern in &self.sinks {
                if &call_name == pattern {
                    return Some(SinkMatch {
                        name: pattern.clone(),
                        rule_id: "CSP-CUSTOM-SINK".to_owned(),
                        vuln_type: VulnType::CodeInjection,
                        severity: Severity::High,
                        dangerous_args: vec![0],
                        dangerous_keywords: Vec::new(),
                        remediation: "Review data flow to this custom sink.".to_owned(),
                    });
                }
            }
        }

        None
    }

    fn patterns(&self) -> Vec<String> {
        self.sinks.clone()
    }
}
