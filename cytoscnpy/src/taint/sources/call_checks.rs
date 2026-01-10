//! Checks for call-based taint sources.

use super::utils::get_call_name;
use crate::taint::types::{TaintInfo, TaintSource};
use ruff_python_ast as ast;
use ruff_text_size::Ranged;

/// Checks if a call expression is a taint source.
pub(crate) fn check_call_source(call: &ast::ExprCall) -> Option<TaintInfo> {
    let line = call.range().start().to_u32() as usize;

    // Get the function name
    if let Some(name) = get_call_name(&call.func) {
        // input() builtin
        if name == "input" {
            return Some(TaintInfo::new(TaintSource::Input, line));
        }

        // os.getenv() or os.environ.get()
        if name == "os.getenv" || name == "getenv" || name == "os.environ.get" {
            return Some(TaintInfo::new(TaintSource::Environment, line));
        }

        // Flask request methods: request.args.get(), request.form.get(), etc.
        if name.starts_with("request.args.")
            || name.starts_with("request.form.")
            || name.starts_with("request.data.")
            || name.starts_with("request.json.")
            || name.starts_with("request.cookies.")
            || name.starts_with("request.files.")
        {
            let attr = name.split('.').nth(1).unwrap_or("args");
            return Some(TaintInfo::new(
                TaintSource::FlaskRequest(attr.to_owned()),
                line,
            ));
        }

        // Django request methods
        if name.starts_with("request.GET.")
            || name.starts_with("request.POST.")
            || name.starts_with("request.COOKIES.")
        {
            let attr = name.split('.').nth(1).unwrap_or("GET");
            return Some(TaintInfo::new(
                TaintSource::DjangoRequest(attr.to_owned()),
                line,
            ));
        }

        // File reads
        // This is not a file extension comparison - we're checking method name suffixes
        #[allow(clippy::case_sensitive_file_extension_comparisons)]
        if name.ends_with(".read") || name.ends_with(".readlines") || name.ends_with(".readline") {
            return Some(TaintInfo::new(TaintSource::FileRead, line));
        }

        // Azure Functions request methods: req.params.get(), req.get_json(), req.get_body()
        // Azure Functions uses 'req' as the conventional parameter name
        if name.starts_with("req.params.")
            || name.starts_with("req.route_params.")
            || name.starts_with("req.headers.")
            || name.starts_with("req.form.")
        {
            let attr = name.split('.').nth(1).unwrap_or("params");
            return Some(TaintInfo::new(
                TaintSource::AzureFunctionsRequest(attr.to_owned()),
                line,
            ));
        }

        // Azure Functions direct methods on HttpRequest
        if name == "req.get_json" || name == "req.get_body" {
            let method = name.split('.').nth(1).unwrap_or("get_json");
            return Some(TaintInfo::new(
                TaintSource::AzureFunctionsRequest(method.to_owned()),
                line,
            ));
        }

        // JSON/YAML load
        if name == "json.load"
            || name == "json.loads"
            || name == "yaml.load"
            || name == "yaml.safe_load"
        {
            return Some(TaintInfo::new(TaintSource::ExternalData, line));
        }
    }

    None
}
