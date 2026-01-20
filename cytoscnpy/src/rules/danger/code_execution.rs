use super::utils::{create_finding, get_call_name, is_arg_literal, SUBPROCESS_INJECTION_MSG};
use crate::rules::ids;
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::Expr;
use ruff_text_size::Ranged;

/// Rule for detecting potentially dangerous `eval()` calls.
pub const META_EVAL: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_EVAL,
    category: super::CAT_CODE_EXEC,
};
/// Rule for detecting potentially dangerous `exec()` calls.
pub const META_EXEC: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_EXEC,
    category: super::CAT_CODE_EXEC,
};
/// Rule for detecting potential command injection in `subprocess` and `os.system`.
pub const META_SUBPROCESS: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_SUBPROCESS,
    category: super::CAT_CODE_EXEC,
};
/// Rule for detecting potential command injection in async subprocesses and popen.
pub const META_ASYNC_SUBPROCESS: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_ASYNC_SUBPROCESS,
    category: super::CAT_CODE_EXEC,
};

/// Rule for detecting potentially dangerous `eval()` calls.
pub struct EvalRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}
impl EvalRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for EvalRule {
    fn name(&self) -> &'static str {
        "EvalRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "eval" {
                    return Some(vec![create_finding(
                        "Avoid using eval",
                        self.metadata,
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
pub struct ExecRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}
impl ExecRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for ExecRule {
    fn name(&self) -> &'static str {
        "ExecRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "exec" {
                    return Some(vec![create_finding(
                        "Avoid using exec",
                        self.metadata,
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

/// Rule for detecting potential command injection in `subprocess` and `os.system`.
pub struct SubprocessRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}
impl SubprocessRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for SubprocessRule {
    fn name(&self) -> &'static str {
        "SubprocessRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "os.system" && !is_arg_literal(&call.arguments.args, 0) {
                    return Some(vec![create_finding(
                        "Potential command injection (os.system with dynamic arg)",
                        self.metadata,
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
                                self.metadata,
                                context,
                                call.range().start(),
                                "CRITICAL",
                            )]);
                        }

                        if let Some(expr) = args_keyword_expr {
                            if !crate::rules::danger::utils::is_literal_expr(expr) {
                                return Some(vec![create_finding(
                                    SUBPROCESS_INJECTION_MSG,
                                    self.metadata,
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

/// Rule for detecting potential command injection in async subprocesses and popen.
pub struct AsyncSubprocessRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}
impl AsyncSubprocessRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for AsyncSubprocessRule {
    fn name(&self) -> &'static str {
        "AsyncSubprocessRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "asyncio.create_subprocess_shell"
                    && !is_arg_literal(&call.arguments.args, 0)
                {
                    return Some(vec![create_finding(
                        "Potential command injection (asyncio.create_subprocess_shell with dynamic args)",
                        self.metadata,
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
                        self.metadata,
                        context,
                        call.range().start(),
                        "HIGH",
                    )]);
                }

                if name == "pty.spawn" && !is_arg_literal(&call.arguments.args, 0) {
                    return Some(vec![create_finding(
                        "Potential command injection (pty.spawn with dynamic args)",
                        self.metadata,
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
