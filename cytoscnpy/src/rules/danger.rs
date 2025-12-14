use crate::rules::{Context, Finding, Rule};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

/// Type inference rules for detecting method misuse on inferred types.
pub mod type_inference;
use type_inference::MethodMisuseRule;

/// Returns a list of all security/danger rules, organized by category.
pub fn get_danger_rules() -> Vec<Box<dyn Rule>> {
    vec![
        // ═══════════════════════════════════════════════════════════════════════
        // Category 1: Code Execution (CSP-D0xx) - Highest Risk
        // ═══════════════════════════════════════════════════════════════════════
        Box::new(EvalRule),       // CSP-D001: eval() usage
        Box::new(ExecRule),       // CSP-D002: exec() usage
        Box::new(SubprocessRule), // CSP-D003: Command injection
        // ═══════════════════════════════════════════════════════════════════════
        // Category 2: Injection Attacks (CSP-D1xx)
        // ═══════════════════════════════════════════════════════════════════════
        Box::new(SqlInjectionRule),    // CSP-D101: SQL injection (ORM)
        Box::new(SqlInjectionRawRule), // CSP-D102: SQL injection (raw)
        Box::new(XSSRule),             // CSP-D103: Cross-site scripting
        // ═══════════════════════════════════════════════════════════════════════
        // Category 3: Deserialization (CSP-D2xx)
        // ═══════════════════════════════════════════════════════════════════════
        Box::new(PickleRule), // CSP-D201: Pickle deserialization
        Box::new(YamlRule),   // CSP-D202: YAML unsafe load
        // ═══════════════════════════════════════════════════════════════════════
        // Category 4: Cryptography (CSP-D3xx)
        // ═══════════════════════════════════════════════════════════════════════
        Box::new(HashlibRule), // CSP-D301: Weak hash algorithms
        // ═══════════════════════════════════════════════════════════════════════
        // Category 5: Network/HTTP (CSP-D4xx)
        // ═══════════════════════════════════════════════════════════════════════
        Box::new(RequestsRule), // CSP-D401: Insecure HTTP requests
        Box::new(SSRFRule),     // CSP-D402: Server-side request forgery
        // ═══════════════════════════════════════════════════════════════════════
        // Category 6: File Operations (CSP-D5xx)
        // ═══════════════════════════════════════════════════════════════════════
        Box::new(PathTraversalRule), // CSP-D501: Path traversal attacks
        Box::new(TarfileExtractionRule), // CSP-D502: Tar extraction vulnerabilities
        Box::new(ZipfileExtractionRule), // CSP-D503: Zip extraction vulnerabilities
        // ═══════════════════════════════════════════════════════════════════════
        // Category 7: Type Safety (CSP-D6xx)
        // ═══════════════════════════════════════════════════════════════════════
        Box::new(MethodMisuseRule::default()), // CSP-D601: Type-based method misuse
    ]
}

struct EvalRule;
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

struct ExecRule;
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

struct PickleRule;
impl Rule for PickleRule {
    fn name(&self) -> &'static str {
        "PickleRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D201" // Default to load
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "pickle.load" {
                    return Some(vec![create_finding(
                        "Avoid using pickle.load (vulnerable to RCE)",
                        "CSP-D201",
                        context,
                        call.range().start(),
                        "CRITICAL",
                    )]);
                } else if name == "pickle.loads" {
                    return Some(vec![create_finding(
                        "Avoid using pickle.loads (vulnerable to RCE)",
                        "CSP-D201-unsafe",
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

struct HashlibRule;
impl Rule for HashlibRule {
    fn name(&self) -> &'static str {
        "HashlibRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D301" // Default to MD5
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "hashlib.md5" {
                    return Some(vec![create_finding(
                        "Weak hashing algorithm (MD5)",
                        "CSP-D301",
                        context,
                        call.range().start(),
                        "MEDIUM",
                    )]);
                }
                if name == "hashlib.sha1" {
                    return Some(vec![create_finding(
                        "Weak hashing algorithm (SHA1)",
                        "CSP-D302",
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
        "CSP-D401"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name.starts_with("requests.") {
                    for keyword in &call.arguments.keywords {
                        if let Some(arg) = &keyword.arg {
                            if arg == "verify" {
                                if let Expr::BooleanLiteral(b) = &keyword.value {
                                    if !b.value {
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
        "CSP-D003"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "os.system" && !is_literal(&call.arguments.args) {
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
                        // Check positional args
                        if !call.arguments.args.is_empty() && !is_literal(&call.arguments.args) {
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
        "CSP-D501"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if (name == "open" || name.starts_with("os.path.") || name.starts_with("shutil."))
                    && !is_literal(&call.arguments.args)
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
        "CSP-D402"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if (name.starts_with("requests.")
                    || name.starts_with("httpx.")
                    || name == "urllib.request.urlopen")
                    && !is_literal(&call.arguments.args)
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
        "CSP-D102"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if (name == "sqlalchemy.text"
                    || name == "pandas.read_sql"
                    || name.ends_with(".objects.raw"))
                    && !is_literal(&call.arguments.args)
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
        "CSP-D103"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if (name == "flask.render_template_string" || name == "jinja2.Markup")
                    && !is_literal(&call.arguments.args)
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
        Expr::StringLiteral(_)
        | Expr::BytesLiteral(_)
        | Expr::NumberLiteral(_)
        | Expr::BooleanLiteral(_)
        | Expr::NoneLiteral(_)
        | Expr::EllipsisLiteral(_) => true,
        Expr::List(list) => list.elts.iter().all(is_literal_expr),
        Expr::Tuple(tuple) => tuple.elts.iter().all(is_literal_expr),
        // f-strings, concatenations, variables, calls, etc. are NOT literal
        Expr::FString(_) | Expr::BinOp(_) | Expr::Name(_) | Expr::Call(_) => false,
        _ => false,
    }
}

fn create_finding(
    msg: &str,
    rule_id: &str,
    context: &Context,
    location: ruff_text_size::TextSize,
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

/// Checks if an expression looks like it's related to tarfile operations.
/// Used to reduce false positives from unrelated .`extractall()` calls.
fn is_likely_tarfile_receiver(receiver: &Expr) -> bool {
    match receiver {
        // tarfile.open(...).extractall() -> receiver is Call to tarfile.open
        Expr::Call(inner_call) => {
            if let Expr::Attribute(inner_attr) = &*inner_call.func {
                // Check for tarfile.open(...)
                inner_attr.attr.as_str() == "open"
                    && matches!(&*inner_attr.value, Expr::Name(n) if n.id.as_str() == "tarfile")
            } else {
                false
            }
        }
        // Variable that might be a TarFile instance
        Expr::Name(name) => {
            let id = name.id.as_str().to_lowercase();
            id == "tarfile" || id.contains("tar") || id == "tf" || id == "t"
        }
        // Attribute access like self.tar_file or module.tar_archive
        Expr::Attribute(attr2) => {
            let attr_id = attr2.attr.as_str().to_lowercase();
            attr_id.contains("tar") || attr_id == "tf"
        }
        _ => false,
    }
}

/// Checks if an expression looks like it's related to zipfile operations.
fn is_likely_zipfile_receiver(receiver: &Expr) -> bool {
    match receiver {
        // zipfile.ZipFile(...).extractall() -> receiver is Call
        Expr::Call(inner_call) => {
            if let Expr::Attribute(inner_attr) = &*inner_call.func {
                // Check for zipfile.ZipFile(...)
                inner_attr.attr.as_str() == "ZipFile"
                    && matches!(&*inner_attr.value, Expr::Name(n) if n.id.as_str() == "zipfile")
            } else if let Expr::Name(name) = &*inner_call.func {
                // Direct ZipFile(...) call
                name.id.as_str() == "ZipFile"
            } else {
                false
            }
        }
        // Variable that might be a ZipFile instance
        Expr::Name(name) => {
            let id = name.id.as_str().to_lowercase();
            id == "zipfile" || id.contains("zip") || id == "zf" || id == "z"
        }
        // Attribute access like self.zip_file
        Expr::Attribute(attr2) => {
            let attr_id = attr2.attr.as_str().to_lowercase();
            attr_id.contains("zip") || attr_id == "zf"
        }
        _ => false,
    }
}

struct TarfileExtractionRule;
impl Rule for TarfileExtractionRule {
    fn name(&self) -> &'static str {
        "TarfileExtractionRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D502"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            // Check for .extractall() call
            if let Expr::Attribute(attr) = &*call.func {
                if attr.attr.as_str() != "extractall" {
                    return None;
                }

                // Heuristic: check if receiver looks like tarfile-related
                let receiver = &attr.value;
                let looks_like_tar = is_likely_tarfile_receiver(receiver);

                // Find 'filter' keyword argument
                let filter_kw = call.arguments.keywords.iter().find_map(|kw| {
                    if kw.arg.as_ref().is_some_and(|a| a == "filter") {
                        Some(&kw.value)
                    } else {
                        None
                    }
                });

                if let Some(filter_expr) = filter_kw {
                    // Filter is present - check if it's a safe literal value
                    let is_safe_literal = if let Expr::StringLiteral(s) = filter_expr {
                        let s_lower = s.value.to_string().to_lowercase();
                        // Python 3.12 doc: filter='data' or 'tar' or 'fully_trusted'
                        s_lower == "data" || s_lower == "tar" || s_lower == "fully_trusted"
                    } else {
                        false
                    };

                    if is_safe_literal {
                        // Safe filter value - no finding
                        return None;
                    }
                    // Filter present but not a recognized safe literal
                    let severity = if looks_like_tar { "MEDIUM" } else { "LOW" };
                    return Some(vec![create_finding(
                        "extractall() with non-literal or unrecognized 'filter' - verify it safely limits extraction paths (recommended: filter='data' or 'tar' in Python 3.12+)",
                        self.code(),
                        context,
                        call.range().start(),
                        severity,
                    )]);
                }
                // No filter argument - high risk for tarfile, medium for unknown
                if looks_like_tar {
                    return Some(vec![create_finding(
                        "Potential Zip Slip: tarfile extractall() without 'filter'. Use filter='data' or 'tar' (Python 3.12+) or validate member paths before extraction",
                        self.code(),
                        context,
                        call.range().start(),
                        "HIGH",
                    )]);
                }
                // Unknown receiver - lower severity to reduce false positives
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

struct ZipfileExtractionRule;
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

                // zipfile.ZipFile has no 'filter' parameter like tarfile
                // The mitigation is to manually check .namelist() before extraction
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
