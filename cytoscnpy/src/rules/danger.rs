use crate::rules::{Context, Finding, Rule};
use rustpython_parser::ast::{self, Expr, Ranged};

/// Type inference rules for detecting method misuse on inferred types.
pub mod type_inference;
use type_inference::MethodMisuseRule;

/// Returns a list of all security/danger rules.
pub fn get_danger_rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(EvalRule),
        Box::new(ExecRule),
        Box::new(PickleRule),
        Box::new(YamlRule),
        Box::new(HashlibRule),
        Box::new(RequestsRule),
        Box::new(SqlInjectionRule),
        Box::new(SubprocessRule),
        Box::new(PathTraversalRule),
        Box::new(SSRFRule),
        Box::new(SqlInjectionRawRule),
        Box::new(SqlInjectionRawRule),
        Box::new(XSSRule),
        Box::new(MethodMisuseRule::default()),
    ]
}

struct EvalRule;
impl Rule for EvalRule {
    fn name(&self) -> &'static str {
        "EvalRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D201"
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

struct ExecRule;
impl Rule for ExecRule {
    fn name(&self) -> &'static str {
        "ExecRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D202"
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

struct PickleRule;
impl Rule for PickleRule {
    fn name(&self) -> &'static str {
        "PickleRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D203" // Default to load
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "pickle.load" {
                    return Some(vec![create_finding(
                        "Avoid using pickle.load (vulnerable to RCE)",
                        "CSP-D203",
                        context,
                        call.range().start(),
                        "CRITICAL",
                    )]);
                } else if name == "pickle.loads" {
                    return Some(vec![create_finding(
                        "Avoid using pickle.loads (vulnerable to RCE)",
                        "CSP-D204",
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

struct YamlRule;
impl Rule for YamlRule {
    fn name(&self) -> &'static str {
        "YamlRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D205"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "yaml.load" {
                    let mut is_safe = false;
                    for keyword in &call.keywords {
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

struct HashlibRule;
impl Rule for HashlibRule {
    fn name(&self) -> &'static str {
        "HashlibRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D206" // Default to MD5
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "hashlib.md5" {
                    return Some(vec![create_finding(
                        "Weak hashing algorithm (MD5)",
                        "CSP-D206",
                        context,
                        call.range().start(),
                        "MEDIUM",
                    )]);
                }
                if name == "hashlib.sha1" {
                    return Some(vec![create_finding(
                        "Weak hashing algorithm (SHA1)",
                        "CSP-D207",
                        context,
                        call.range().start(),
                        "MEDIUM",
                    )]);
                }
            }
        }
        None
    }
}

struct RequestsRule;
impl Rule for RequestsRule {
    fn name(&self) -> &'static str {
        "RequestsRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D208"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name.starts_with("requests.") {
                    for keyword in &call.keywords {
                        if let Some(arg) = &keyword.arg {
                            if arg == "verify" {
                                if let Expr::Constant(c) = &keyword.value {
                                    if let ast::Constant::Bool(false) = c.value {
                                        return Some(vec![create_finding(
                                            "SSL verification disabled (verify=False)",
                                            self.code(),
                                            context,
                                            call.range().start(),
                                            "HIGH",
                                        )]);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }
}

struct SubprocessRule;
impl Rule for SubprocessRule {
    fn name(&self) -> &'static str {
        "SubprocessRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D212"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "os.system" && !is_literal(&call.args) {
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

                    for keyword in &call.keywords {
                        if let Some(arg) = &keyword.arg {
                            match arg.as_str() {
                                "shell" => {
                                    if let Expr::Constant(c) = &keyword.value {
                                        if let ast::Constant::Bool(true) = c.value {
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
                        // Check positional args
                        if !call.args.is_empty() && !is_literal(&call.args) {
                            return Some(vec![create_finding(
                                SUBPROCESS_INJECTION_MSG,
                                self.code(),
                                context,
                                call.range().start(),
                                "CRITICAL",
                            )]);
                        }

                        // Check keyword args (args=...)
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

struct SqlInjectionRule;
impl Rule for SqlInjectionRule {
    fn name(&self) -> &'static str {
        "SqlInjectionRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D211"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name.ends_with(".execute") || name.ends_with(".executemany") {
                    if let Some(arg) = call.args.first() {
                        if let Expr::JoinedStr(_) = arg {
                            return Some(vec![create_finding(
                                "Potential SQL injection (f-string in execute)",
                                self.code(),
                                context,
                                call.range().start(),
                                "CRITICAL",
                            )]);
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
                        }
                    }
                }
            }
        }
        None
    }
}

struct PathTraversalRule;
impl Rule for PathTraversalRule {
    fn name(&self) -> &'static str {
        "PathTraversalRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D215"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if (name == "open" || name.starts_with("os.path.") || name.starts_with("shutil."))
                    && !is_literal(&call.args)
                {
                    // This is a heuristic, assuming non-literal args might be tainted.
                    // Real taint analysis is needed for high confidence.
                    // For now, we only flag if it looks like user input might be involved (not implemented here)
                    // or just flag all dynamic paths as HIGH risk if we want to be strict.
                    // Given the user request, we should implement a basic check.
                    return Some(vec![create_finding(
                        "Potential path traversal (dynamic file path)",
                        self.code(),
                        context,
                        call.range.start(),
                        "HIGH",
                    )]);
                }
            }
        }
        None
    }
}

struct SSRFRule;
impl Rule for SSRFRule {
    fn name(&self) -> &'static str {
        "SSRFRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D216"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if (name.starts_with("requests.")
                    || name.starts_with("httpx.")
                    || name == "urllib.request.urlopen")
                    && !is_literal(&call.args)
                {
                    return Some(vec![create_finding(
                        "Potential SSRF (dynamic URL)",
                        self.code(),
                        context,
                        call.range.start(),
                        "CRITICAL",
                    )]);
                }
            }
        }
        None
    }
}

struct SqlInjectionRawRule;
impl Rule for SqlInjectionRawRule {
    fn name(&self) -> &'static str {
        "SqlInjectionRawRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D217"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if (name == "sqlalchemy.text"
                    || name == "pandas.read_sql"
                    || name.ends_with(".objects.raw"))
                    && !is_literal(&call.args)
                {
                    return Some(vec![create_finding(
                        "Potential SQL injection (dynamic raw SQL)",
                        self.code(),
                        context,
                        call.range.start(),
                        "CRITICAL",
                    )]);
                }
            }
        }
        None
    }
}

struct XSSRule;
impl Rule for XSSRule {
    fn name(&self) -> &'static str {
        "XSSRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D226"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if (name == "flask.render_template_string" || name == "jinja2.Markup")
                    && !is_literal(&call.args)
                {
                    return Some(vec![create_finding(
                        "Potential XSS (dynamic template/markup)",
                        self.code(),
                        context,
                        call.range.start(),
                        "CRITICAL",
                    )]);
                }
            }
        }
        None
    }
}

// Helper functions

/// Message for subprocess command injection findings
const SUBPROCESS_INJECTION_MSG: &str =
    "Potential command injection (subprocess with shell=True and dynamic args)";

fn get_call_name(func: &Expr) -> Option<String> {
    match func {
        Expr::Name(node) => Some(node.id.to_string()),
        Expr::Attribute(node) => {
            if let Expr::Name(value) = &*node.value {
                Some(format!("{}.{}", value.id, node.attr))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn is_literal(args: &[Expr]) -> bool {
    if let Some(arg) = args.first() {
        is_literal_expr(arg)
    } else {
        true // No args is "literal" in the sense of safe
    }
}

/// Check if a single expression is a literal (constant value).
/// Returns false for dynamic values like variables, f-strings, concatenations, etc.
fn is_literal_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Constant(_) => true,
        Expr::List(list) => list.elts.iter().all(is_literal_expr),
        Expr::Tuple(tuple) => tuple.elts.iter().all(is_literal_expr),
        // f-strings, concatenations, variables, calls, etc. are NOT literal
        Expr::JoinedStr(_) | Expr::BinOp(_) | Expr::Name(_) | Expr::Call(_) => false,
        _ => false,
    }
}

fn create_finding(
    msg: &str,
    rule_id: &str,
    context: &Context,
    location: rustpython_parser::text_size::TextSize,
    severity: &str,
) -> Finding {
    let line = context.line_index.line_index(location);
    Finding {
        message: msg.to_owned(),
        rule_id: rule_id.to_owned(),
        file: context.filename.clone(),
        line,
        col: 0,
        severity: severity.to_owned(),
    }
}
