// Note: We check Python method names like `.parse`, `.fromstring` which clippy
// incorrectly flags as file extension comparisons
#![allow(clippy::case_sensitive_file_extension_comparisons)]

use super::utils::{
    create_finding, get_call_name, is_arg_literal, is_likely_tarfile_receiver,
    is_likely_zipfile_receiver, is_literal_expr, SUBPROCESS_INJECTION_MSG,
};
use crate::rules::{Context, Finding, Rule};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

/// Rule for detecting potentially dangerous `eval()` calls.
pub struct EvalRule;
impl Rule for EvalRule {
    fn name(&self) -> &'static str {
        "EvalRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D001"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "eval" {
                    return Some(vec![create_finding(
                        "Avoid using eval",
                        self.code(),
                        context,
                        call.range().start(),
                        "HIGH",
                    )]);
                }
            }
        }
        None
    }
}

/// Rule for detecting potentially dangerous `exec()` calls.
pub struct ExecRule;
impl Rule for ExecRule {
    fn name(&self) -> &'static str {
        "ExecRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D002"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "exec" {
                    return Some(vec![create_finding(
                        "Avoid using exec",
                        self.code(),
                        context,
                        call.range().start(),
                        "HIGH",
                    )]);
                }
            }
        }
        None
    }
}

/// Rule for detecting insecure usage of `pickle` and similar deserialization modules.
pub struct PickleRule;
impl Rule for PickleRule {
    fn name(&self) -> &'static str {
        "PickleRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D201"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if (name.starts_with("pickle.")
                    || name.starts_with("cPickle.")
                    || name.starts_with("dill.")
                    || name.starts_with("shelve.")
                    || name.starts_with("jsonpickle.")
                    || name == "pandas.read_pickle")
                    && (name.contains("load")
                        || name.contains("Unpickler")
                        || name == "shelve.open"
                        || name == "shelve.DbfilenameShelf"
                        || name.contains("decode")
                        || name == "pandas.read_pickle")
                {
                    return Some(vec![create_finding(
                            "Avoid using pickle/dill/shelve/jsonpickle/pandas.read_pickle (vulnerable to RCE on untrusted data)",
                            self.code(),
                            context,
                            call.range().start(),
                            "CRITICAL",
                        )]);
                }
            }
        }
        None
    }
}

/// Rule for detecting unsafe YAML loading.
pub struct YamlRule;
impl Rule for YamlRule {
    fn name(&self) -> &'static str {
        "YamlRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D202"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "yaml.load" {
                    let mut is_safe = false;
                    for keyword in &call.arguments.keywords {
                        if let Some(arg) = &keyword.arg {
                            if arg == "Loader" {
                                if let Expr::Name(n) = &keyword.value {
                                    if n.id.as_str() == "SafeLoader" {
                                        is_safe = true;
                                    }
                                }
                            }
                        }
                    }
                    if !is_safe {
                        return Some(vec![create_finding(
                            "Use yaml.safe_load or Loader=SafeLoader",
                            self.code(),
                            context,
                            call.range().start(),
                            "HIGH",
                        )]);
                    }
                }
            }
        }
        None
    }
}

/// Rule for detecting potential command injection in `subprocess` and `os.system`.
pub struct SubprocessRule;
impl Rule for SubprocessRule {
    fn name(&self) -> &'static str {
        "SubprocessRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D003"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "os.system" && !is_arg_literal(&call.arguments.args, 0) {
                    return Some(vec![create_finding(
                        "Potential command injection (os.system with dynamic arg)",
                        self.code(),
                        context,
                        call.range().start(),
                        "CRITICAL",
                    )]);
                }
                if name.starts_with("subprocess.") {
                    let mut is_shell_true = false;
                    let mut args_keyword_expr: Option<&Expr> = None;

                    for keyword in &call.arguments.keywords {
                        if let Some(arg) = &keyword.arg {
                            match arg.as_str() {
                                "shell" => {
                                    if let Expr::BooleanLiteral(b) = &keyword.value {
                                        if b.value {
                                            is_shell_true = true;
                                        }
                                    }
                                }
                                "args" => {
                                    args_keyword_expr = Some(&keyword.value);
                                }
                                _ => {}
                            }
                        }
                    }

                    if is_shell_true {
                        if !call.arguments.args.is_empty()
                            && !is_arg_literal(&call.arguments.args, 0)
                        {
                            return Some(vec![create_finding(
                                SUBPROCESS_INJECTION_MSG,
                                self.code(),
                                context,
                                call.range().start(),
                                "CRITICAL",
                            )]);
                        }

                        if let Some(expr) = args_keyword_expr {
                            if !is_literal_expr(expr) {
                                return Some(vec![create_finding(
                                    SUBPROCESS_INJECTION_MSG,
                                    self.code(),
                                    context,
                                    call.range().start(),
                                    "CRITICAL",
                                )]);
                            }
                        }
                    }
                }
            }
        }
        None
    }
}

/// Rule for detecting potential SQL injection in ORM-like `execute` calls.
pub struct SqlInjectionRule;
impl Rule for SqlInjectionRule {
    fn name(&self) -> &'static str {
        "SqlInjectionRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D101"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name.ends_with(".execute") || name.ends_with(".executemany") {
                    if let Some(arg) = call.arguments.args.first() {
                        if let Expr::FString(_) = arg {
                            return Some(vec![create_finding(
                                "Potential SQL injection (f-string in execute)",
                                self.code(),
                                context,
                                call.range().start(),
                                "CRITICAL",
                            )]);
                        }
                        if let Expr::Call(inner_call) = arg {
                            if let Expr::Attribute(attr) = &*inner_call.func {
                                if attr.attr.as_str() == "format" {
                                    return Some(vec![create_finding(
                                        "Potential SQL injection (str.format in execute)",
                                        self.code(),
                                        context,
                                        call.range().start(),
                                        "CRITICAL",
                                    )]);
                                }
                            }
                        }
                        if let Expr::BinOp(binop) = arg {
                            if matches!(binop.op, ast::Operator::Add) {
                                return Some(vec![create_finding(
                                    "Potential SQL injection (string concatenation in execute)",
                                    self.code(),
                                    context,
                                    call.range().start(),
                                    "CRITICAL",
                                )]);
                            }
                            if matches!(binop.op, ast::Operator::Mod) {
                                return Some(vec![create_finding(
                                    "Potential SQL injection (% formatting in execute)",
                                    self.code(),
                                    context,
                                    call.range().start(),
                                    "CRITICAL",
                                )]);
                            }
                        }
                    }
                }
            }
        }
        None
    }
}

/// Rule for detecting potential SQL injection in raw SQL queries.
pub struct SqlInjectionRawRule;
impl Rule for SqlInjectionRawRule {
    fn name(&self) -> &'static str {
        "SqlInjectionRawRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D102"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            let mut is_sqli = false;

            if let Some(name) = get_call_name(&call.func) {
                if (name == "sqlalchemy.text"
                    || name == "pandas.read_sql"
                    || name.ends_with(".objects.raw")
                    || name.contains("Template.substitute")
                    || name.starts_with("jinjasql."))
                    && !is_arg_literal(&call.arguments.args, 0)
                {
                    is_sqli = true;
                }
            }

            // Comment 1: Handle Template(...).substitute() and JinjaSql.prepare_query()
            if !is_sqli {
                if let Expr::Attribute(attr) = &*call.func {
                    let attr_name = attr.attr.as_str();
                    if attr_name == "substitute" || attr_name == "safe_substitute" {
                        // Check if receiver is a Call to Template()
                        if let Expr::Call(inner_call) = &*attr.value {
                            if let Some(inner_name) = get_call_name(&inner_call.func) {
                                if inner_name == "Template" || inner_name == "string.Template" {
                                    // If either the template itself or the substitution values are dynamic
                                    if !is_arg_literal(&inner_call.arguments.args, 0)
                                        || !is_arg_literal(&call.arguments.args, 0)
                                        || call
                                            .arguments
                                            .keywords
                                            .iter()
                                            .any(|k| !is_literal_expr(&k.value))
                                    {
                                        is_sqli = true;
                                    }
                                }
                            }
                        }
                    } else if attr_name == "prepare_query" {
                        // Check if receiver is instantiated from JinjaSql or called on a likely JinjaSql object
                        let is_jinja = match &*attr.value {
                            Expr::Call(inner) => get_call_name(&inner.func)
                                .is_some_and(|n| n == "JinjaSql" || n == "jinjasql.JinjaSql"),
                            Expr::Name(n) => {
                                let id = n.id.as_str().to_lowercase();
                                id == "j" || id.contains("jinjasql")
                            }
                            _ => false,
                        };
                        if is_jinja && !is_arg_literal(&call.arguments.args, 0) {
                            is_sqli = true;
                        }
                    }
                }
            }

            if is_sqli {
                return Some(vec![create_finding(
                    "Potential SQL injection (dynamic raw SQL)",
                    self.code(),
                    context,
                    call.range().start(),
                    "CRITICAL",
                )]);
            }
        }
        None
    }
}

/// Rule for detecting potential Cross-Site Scripting (XSS) vulnerabilities.
pub struct XSSRule;
impl Rule for XSSRule {
    fn name(&self) -> &'static str {
        "XSSRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D103"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if (name == "flask.render_template_string"
                    || name == "jinja2.Markup"
                    || name == "flask.Markup"
                    || name.starts_with("django.utils.html.format_html")
                    || name == "format_html"
                    || name.ends_with(".HTMLResponse")
                    || name == "HTMLResponse")
                    && (!is_arg_literal(&call.arguments.args, 0)
                        || call.arguments.keywords.iter().any(|k| {
                            k.arg.as_ref().is_some_and(|a| {
                                let s = a.as_str();
                                s == "content" || s == "body" || s == "source" || s == "template"
                            }) && !is_literal_expr(&k.value)
                        }))
                {
                    return Some(vec![create_finding(
                        "Potential XSS (dynamic template/markup)",
                        self.code(),
                        context,
                        call.range().start(),
                        "CRITICAL",
                    )]);
                }
            }
        }
        None
    }
}

/// Rule for detecting insecure XML parsing.
pub struct XmlRule;
impl Rule for XmlRule {
    fn name(&self) -> &'static str {
        "XmlRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D104"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name.ends_with(".parse")
                    || name.ends_with(".iterparse")
                    || name.ends_with(".fromstring")
                    || name.ends_with(".XML")
                    || name.ends_with(".parseString")
                    || name.ends_with(".make_parser")
                    || name.ends_with(".RestrictedElement")
                    || name.ends_with(".GlobalParserTLS")
                    || name.ends_with(".getDefaultParser")
                    || name.ends_with(".check_docinfo")
                {
                    if name.contains("lxml") {
                        return Some(vec![create_finding(
                            "Potential XML XXE vulnerability in lxml. Use defusedxml.lxml or configure parser safely (resolve_entities=False)",
                            self.code(),
                            context,
                            call.range().start(),
                            "HIGH",
                        )]);
                    }

                    if name.contains("xml.etree")
                        || name.contains("ElementTree")
                        || name.contains("minidom")
                        || name.contains("sax")
                        || name.contains("pulldom")
                        || name.contains("expatbuilder")
                        || name.starts_with("ET.")
                        || name.starts_with("etree.")
                    {
                        let msg = if name.contains("minidom") {
                            "Potential XML DoS (Billion Laughs) in xml.dom.minidom. Use defusedxml.minidom"
                        } else if name.contains("sax") {
                            "Potential XML XXE/DoS in xml.sax. Use defusedxml.sax"
                        } else {
                            "Potential XML vulnerability (XXE/Billion Laughs) - use defusedxml or ensure parser is configured securely"
                        };

                        return Some(vec![create_finding(
                            msg,
                            self.code(),
                            context,
                            call.range().start(),
                            "MEDIUM",
                        )]);
                    }
                }
            }
        }
        None
    }
}

/// Rule for detecting insecure `tarfile` extractions (Zip Slip).
pub struct TarfileExtractionRule;
impl Rule for TarfileExtractionRule {
    fn name(&self) -> &'static str {
        "TarfileExtractionRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D502"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Expr::Attribute(attr) = &*call.func {
                if attr.attr.as_str() != "extractall" {
                    return None;
                }

                let receiver = &attr.value;
                let looks_like_tar = is_likely_tarfile_receiver(receiver);

                let filter_kw = call.arguments.keywords.iter().find_map(|kw| {
                    if kw.arg.as_ref().is_some_and(|a| a == "filter") {
                        Some(&kw.value)
                    } else {
                        None
                    }
                });

                if let Some(filter_expr) = filter_kw {
                    let is_safe_literal = if let Expr::StringLiteral(s) = filter_expr {
                        let s_lower = s.value.to_string().to_lowercase();
                        s_lower == "data" || s_lower == "tar" || s_lower == "fully_trusted"
                    } else {
                        false
                    };

                    if is_safe_literal {
                        return None;
                    }
                    let severity = if looks_like_tar { "MEDIUM" } else { "LOW" };
                    return Some(vec![create_finding(
                        "extractall() with non-literal or unrecognized 'filter' - verify it safely limits extraction paths (recommended: filter='data' or 'tar' in Python 3.12+)",
                        self.code(),
                        context,
                        call.range().start(),
                        severity,
                    )]);
                }
                if looks_like_tar {
                    return Some(vec![create_finding(
                        "Potential Zip Slip: tarfile extractall() without 'filter'. Use filter='data' or 'tar' (Python 3.12+) or validate member paths before extraction",
                        self.code(),
                        context,
                        call.range().start(),
                        "HIGH",
                    )]);
                }
                return Some(vec![create_finding(
                    "Possible unsafe extractall() call without 'filter'. If this is a tarfile, use filter='data' or 'tar' (Python 3.12+)",
                    self.code(),
                    context,
                    call.range().start(),
                    "MEDIUM",
                )]);
            }
        }
        None
    }
}

/// Rule for detecting insecure `zipfile` extractions (Zip Slip).
pub struct ZipfileExtractionRule;
impl Rule for ZipfileExtractionRule {
    fn name(&self) -> &'static str {
        "ZipfileExtractionRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D503"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Expr::Attribute(attr) = &*call.func {
                if attr.attr.as_str() != "extractall" {
                    return None;
                }

                let receiver = &attr.value;
                let looks_like_zip = is_likely_zipfile_receiver(receiver);

                if looks_like_zip {
                    return Some(vec![create_finding(
                        "Potential Zip Slip: zipfile extractall() without path validation. Check ZipInfo.filename for '..' and absolute paths before extraction",
                        self.code(),
                        context,
                        call.range().start(),
                        "HIGH",
                    )]);
                }
            }
        }
        None
    }
}

/// Rule for detecting potential command injection in async subprocesses and popen.
pub struct AsyncSubprocessRule;
impl Rule for AsyncSubprocessRule {
    fn name(&self) -> &'static str {
        "AsyncSubprocessRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D901"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "asyncio.create_subprocess_shell"
                    && !is_arg_literal(&call.arguments.args, 0)
                {
                    return Some(vec![create_finding(
                        "Potential command injection (asyncio.create_subprocess_shell with dynamic args)",
                        self.code(),
                        context,
                        call.range().start(),
                        "CRITICAL",
                    )]);
                }

                if (name == "os.popen"
                    || name == "os.popen2"
                    || name == "os.popen3"
                    || name == "os.popen4")
                    && !is_arg_literal(&call.arguments.args, 0)
                {
                    return Some(vec![create_finding(
                        "Potential command injection (os.popen with dynamic args). Use subprocess module instead.",
                        self.code(),
                        context,
                        call.range().start(),
                        "HIGH",
                    )]);
                }

                if name == "pty.spawn" && !is_arg_literal(&call.arguments.args, 0) {
                    return Some(vec![create_finding(
                        "Potential command injection (pty.spawn with dynamic args)",
                        self.code(),
                        context,
                        call.range().start(),
                        "HIGH",
                    )]);
                }
            }
        }
        None
    }
}

/// Rule for detecting insecure deserialization of machine learning models.
pub struct ModelDeserializationRule;
impl Rule for ModelDeserializationRule {
    fn name(&self) -> &'static str {
        "ModelDeserializationRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D902"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "torch.load" {
                    let has_weights_only = call.arguments.keywords.iter().any(|kw| {
                        if let Some(arg) = &kw.arg {
                            if arg == "weights_only" {
                                if let Expr::BooleanLiteral(b) = &kw.value {
                                    return b.value;
                                }
                            }
                        }
                        false
                    });
                    if !has_weights_only {
                        return Some(vec![create_finding(
                            "torch.load() without weights_only=True can execute arbitrary code. Use weights_only=True or torch.safe_load().",
                            self.code(),
                            context,
                            call.range().start(),
                            "CRITICAL",
                        )]);
                    }
                }

                if name == "joblib.load" {
                    return Some(vec![create_finding(
                        "joblib.load() can execute arbitrary code. Ensure the model source is trusted.",
                        self.code(),
                        context,
                        call.range().start(),
                        "HIGH",
                    )]);
                }

                if name == "keras.models.load_model"
                    || name == "tf.keras.models.load_model"
                    || name == "load_model"
                    || name == "keras.load_model"
                {
                    let has_safe_mode = call.arguments.keywords.iter().any(|kw| {
                        if let Some(arg) = &kw.arg {
                            if arg == "safe_mode" {
                                if let Expr::BooleanLiteral(b) = &kw.value {
                                    return b.value;
                                }
                            }
                        }
                        false
                    });
                    if !has_safe_mode {
                        return Some(vec![create_finding(
                            "keras.models.load_model() without safe_mode=True can load Lambda layers with arbitrary code.",
                            self.code(),
                            context,
                            call.range().start(),
                            "HIGH",
                        )]);
                    }
                }
            }
        }
        None
    }
}
