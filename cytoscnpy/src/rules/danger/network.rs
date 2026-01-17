use super::utils::{create_finding, get_call_name, is_literal};
use crate::rules::{Context, Finding, Rule};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

pub struct RequestsRule;
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

pub struct SSRFRule;
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
                if name.starts_with("requests.")
                    || name.starts_with("httpx.")
                    || name == "urllib.request.urlopen"
                {
                    let mut findings = Vec::new();

                    // Case 1: Positional arguments
                    if !call.arguments.args.is_empty() {
                        if name.ends_with(".request") {
                            // For .request(method, url, ...), check 2nd arg (index 1)
                            if call.arguments.args.len() >= 2
                                && !crate::rules::danger::utils::is_literal_expr(
                                    &call.arguments.args[1],
                                )
                            {
                                findings.push(create_finding(
                                    "Potential SSRF (dynamic URL in positional arg 2)",
                                    self.code(),
                                    context,
                                    call.arguments.args[1].range().start(),
                                    "CRITICAL",
                                ));
                            }
                        } else {
                            // For .get(url, ...), .post(url, ...), check 1st arg via is_literal check
                            // Note: is_literal checks if ALL args are literal. If any is dynamic, logic assumes risk.
                            // Ideally we just check the first arg for exactness, but keeping existing heuristic for now
                            // unless strictly asked only for .request change.
                            // The guard !is_literal(&call.arguments.args) covers the "any dynamic arg" case.
                            if !is_literal(&call.arguments.args) {
                                findings.push(create_finding(
                                    "Potential SSRF (dynamic URL in positional arg)",
                                    self.code(),
                                    context,
                                    call.range.start(),
                                    "CRITICAL",
                                ));
                            }
                        }
                    }

                    // Case 2: Keyword arguments (Always check)
                    for keyword in &call.arguments.keywords {
                        if let Some(arg) = &keyword.arg {
                            let arg_s = arg.as_str();
                            if matches!(arg_s, "url" | "uri" | "address") {
                                if !crate::rules::danger::utils::is_literal_expr(&keyword.value) {
                                    findings.push(create_finding(
                                        format!("Potential SSRF (dynamic URL in '{}' arg)", arg_s)
                                            .as_str(),
                                        self.code(),
                                        context,
                                        keyword.value.range().start(),
                                        "CRITICAL",
                                    ));
                                }
                            }
                        }
                    }

                    if !findings.is_empty() {
                        return Some(findings);
                    }
                }
            }
        }
        None
    }
}

pub struct HardcodedBindAllInterfacesRule;
impl Rule for HardcodedBindAllInterfacesRule {
    fn name(&self) -> &'static str {
        "HardcodedBindAllInterfacesRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D404"
    }
    fn enter_stmt(&mut self, stmt: &Stmt, context: &Context) -> Option<Vec<Finding>> {
        match stmt {
            Stmt::Assign(assign) => {
                let is_host_bind = assign.targets.iter().any(|t| {
                    if let Expr::Name(n) = t {
                        let name = n.id.to_lowercase();
                        name.contains("host") || name.contains("bind") || name == "listen_addr"
                    } else {
                        false
                    }
                });
                if is_host_bind {
                    if let Expr::StringLiteral(s) = &*assign.value {
                        let val = s.value.to_string();
                        if val == "0.0.0.0" || val == "::" {
                            return Some(vec![create_finding(
                                "Possible hardcoded binding to all interfaces (0.0.0.0 or ::)",
                                self.code(),
                                context,
                                assign.value.range().start(),
                                "MEDIUM",
                            )]);
                        }
                    }
                }
            }
            Stmt::AnnAssign(any_assign) => {
                if let Expr::Name(n) = &*any_assign.target {
                    let name = n.id.to_lowercase();
                    if name.contains("host") || name.contains("bind") || name == "listen_addr" {
                        if let Some(value) = &any_assign.value {
                            if let Expr::StringLiteral(s) = &**value {
                                let val = s.value.to_string();
                                if val == "0.0.0.0" || val == "::" {
                                    return Some(vec![create_finding(
                                        "Possible hardcoded binding to all interfaces (0.0.0.0 or ::)",
                                        self.code(),
                                        context,
                                        value.range().start(),
                                        "MEDIUM",
                                    )]);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        None
    }

    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            // Check keywords for host/bind
            for kw in &call.arguments.keywords {
                if let Some(arg_name) = &kw.arg {
                    if arg_name == "host" || arg_name == "bind" {
                        if let Expr::StringLiteral(s) = &kw.value {
                            let val = s.value.to_string();
                            if val == "0.0.0.0" || val == "::" {
                                return Some(vec![create_finding(
                                    "Possible hardcoded binding to all interfaces (0.0.0.0 or ::)",
                                    self.code(),
                                    context,
                                    kw.value.range().start(),
                                    "MEDIUM",
                                )]);
                            }
                        }
                    }
                }
            }
            // Check positional socket.bind(("0.0.0.0", 80))
            if let Some(name) = get_call_name(&call.func) {
                if (name == "bind" || name.ends_with(".bind")) && !call.arguments.args.is_empty() {
                    if let Expr::Tuple(t) = &call.arguments.args[0] {
                        if !t.elts.is_empty() {
                            if let Expr::StringLiteral(s) = &t.elts[0] {
                                let val = s.value.to_string();
                                if val == "0.0.0.0" || val == "::" {
                                    return Some(vec![create_finding(
                                        "Possible hardcoded binding to all interfaces (0.0.0.0 or ::)",
                                        self.code(),
                                        context,
                                        t.elts[0].range().start(),
                                        "MEDIUM",
                                    )]);
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

pub struct RequestWithoutTimeoutRule;
impl Rule for RequestWithoutTimeoutRule {
    fn name(&self) -> &'static str {
        "RequestWithoutTimeoutRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D405"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if (name.starts_with("requests.") || name.starts_with("httpx."))
                    && (name.ends_with(".get")
                        || name.ends_with(".post")
                        || name.ends_with(".put")
                        || name.ends_with(".delete")
                        || name.ends_with(".head")
                        || name.ends_with(".patch")
                        || name.ends_with(".request"))
                {
                    let mut bad_timeout = true;
                    for kw in &call.arguments.keywords {
                        if kw.arg.as_ref().is_some_and(|a| a == "timeout") {
                            bad_timeout = match &kw.value {
                                Expr::NoneLiteral(_) => true,
                                Expr::BooleanLiteral(b) => !b.value,
                                Expr::NumberLiteral(n) => match &n.value {
                                    ast::Number::Int(i) => i.to_string() == "0",
                                    ast::Number::Float(f) => *f == 0.0,
                                    ast::Number::Complex { .. } => false,
                                },
                                _ => false,
                            };
                            if !bad_timeout {
                                break;
                            }
                        }
                    }
                    if bad_timeout {
                        return Some(vec![create_finding(
                            "Request call without timeout or with an unsafe timeout (None, 0, False). This can cause the process to hang indefinitely.",
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

/// Check for network and SSL-related insecure patterns (B309, B310, B312, B321, B323)
pub fn check_network_and_ssl(
    name: &str,
    call: &ast::ExprCall,
    context: &Context,
) -> Option<Finding> {
    // B309: HTTPSConnection
    if name == "httplib.HTTPSConnection"
        || name == "http.client.HTTPSConnection"
        || name == "six.moves.http_client.HTTPSConnection"
    {
        let has_context = call
            .arguments
            .keywords
            .iter()
            .any(|k| k.arg.as_ref().is_some_and(|a| a == "context"));
        if !has_context {
            return Some(create_finding(
                "Use of HTTPSConnection without a context is insecure in some Python versions.",
                "CSP-D408",
                context,
                call.range().start(),
                "MEDIUM",
            ));
        }
    }
    // B310: urllib
    if name.starts_with("urllib.urlopen")
        || name.starts_with("urllib.request.urlopen")
        || name.starts_with("urllib2.urlopen")
        || name.starts_with("six.moves.urllib.request.urlopen")
        || name.contains("urlretrieve")
        || name.contains("URLopener")
    {
        return Some(create_finding(
            "Audit url open for permitted schemes. Allowing file: or custom schemes is dangerous.",
            "CSP-D406",
            context,
            call.range().start(),
            "MEDIUM",
        ));
    }
    // B312: telnetlib call
    if name.starts_with("telnetlib.") {
        return Some(create_finding(
            "Telnet-related functions are being called. Telnet is insecure.",
            "CSP-D005",
            context,
            call.range().start(),
            "HIGH",
        ));
    }
    // B321: ftplib call
    if name.starts_with("ftplib.") {
        return Some(create_finding(
            "FTP-related functions are being called. FTP is insecure.",
            "CSP-D006",
            context,
            call.range().start(),
            "HIGH",
        ));
    }
    // B323: unverified context
    if name == "ssl._create_unverified_context" {
        return Some(create_finding(
            "Use of potentially insecure ssl._create_unverified_context.",
            "CSP-D407",
            context,
            call.range().start(),
            "MEDIUM",
        ));
    }
    // Extension: ssl.wrap_socket detection
    if name == "ssl.wrap_socket" {
        return Some(create_finding(
            "Use of ssl.wrap_socket is deprecated and often insecure. Use ssl.create_default_context().wrap_socket() instead.",
            "CSP-D409",
            context,
            call.range().start(),
            "MEDIUM",
        ));
    }
    None
}
