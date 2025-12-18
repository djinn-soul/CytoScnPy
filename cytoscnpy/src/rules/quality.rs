use crate::config::Config;
use crate::rules::{Context, Finding, Rule};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

/// Returns a list of all quality rules based on configuration.
pub fn get_quality_rules(config: &Config) -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(MutableDefaultArgumentRule),
        Box::new(BareExceptRule),
        Box::new(DangerousComparisonRule),
        Box::new(ArgumentCountRule::new(
            config.cytoscnpy.max_args.unwrap_or(5),
        )),
        Box::new(FunctionLengthRule::new(
            config.cytoscnpy.max_lines.unwrap_or(50),
        )),
        Box::new(ComplexityRule::new(
            config.cytoscnpy.complexity.unwrap_or(10),
        )),
        Box::new(NestingRule::new(config.cytoscnpy.nesting.unwrap_or(3))),
    ]
}

struct MutableDefaultArgumentRule;
impl Rule for MutableDefaultArgumentRule {
    fn name(&self) -> &'static str {
        "MutableDefaultArgumentRule"
    }
    fn code(&self) -> &'static str {
        "CSP-L001"
    }
    fn enter_stmt(&mut self, stmt: &Stmt, context: &Context) -> Option<Vec<Finding>> {
        let parameters = match stmt {
            Stmt::FunctionDef(f) => &f.parameters,
            _ => return None,
        };

        let mut findings = Vec::new();

        let check_arg = |arg: &ast::ParameterWithDefault, findings: &mut Vec<Finding>| {
            if let Some(default) = &arg.default {
                if is_mutable(default) {
                    findings.push(create_finding(
                        "Mutable default argument (use None and check inside function)",
                        self.code(),
                        context,
                        default.range().start(),
                        "MEDIUM",
                    ));
                }
            }
        };

        for arg in &parameters.posonlyargs {
            check_arg(arg, &mut findings);
        }
        for arg in &parameters.args {
            check_arg(arg, &mut findings);
        }
        for arg in &parameters.kwonlyargs {
            check_arg(arg, &mut findings);
        }
        if findings.is_empty() {
            None
        } else {
            Some(findings)
        }
    }
}

fn is_mutable(expr: &Expr) -> bool {
    match expr {
        Expr::List(_) | Expr::Dict(_) | Expr::Set(_) => true,
        Expr::Call(call) => {
            if let Expr::Name(name) = &*call.func {
                matches!(name.id.as_str(), "list" | "dict" | "set")
            } else {
                false
            }
        }
        _ => false,
    }
}

struct BareExceptRule;
impl Rule for BareExceptRule {
    fn name(&self) -> &'static str {
        "BareExceptRule"
    }
    fn code(&self) -> &'static str {
        "CSP-L002"
    }
    fn enter_stmt(&mut self, stmt: &Stmt, context: &Context) -> Option<Vec<Finding>> {
        if let Stmt::Try(t) = stmt {
            for handler in &t.handlers {
                let ast::ExceptHandler::ExceptHandler(h) = handler;
                if h.type_.is_none() {
                    return Some(vec![create_finding(
                        "Bare except block (catch specific exceptions)",
                        self.code(),
                        context,
                        h.range().start(),
                        "LOW",
                    )]);
                }
            }
        }
        None
    }
}

struct DangerousComparisonRule;
impl Rule for DangerousComparisonRule {
    fn name(&self) -> &'static str {
        "DangerousComparisonRule"
    }
    fn code(&self) -> &'static str {
        "CSP-L003"
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Compare(comp) = expr {
            for (i, comparator) in comp.comparators.iter().enumerate() {
                let op = &comp.ops[i];
                if matches!(op, ast::CmpOp::Eq | ast::CmpOp::NotEq) {
                    // Check for BooleanLiteral or NoneLiteral
                    let is_dangerous =
                        matches!(comparator, Expr::BooleanLiteral(_) | Expr::NoneLiteral(_));
                    if is_dangerous {
                        return Some(vec![create_finding(
                            "Dangerous comparison to True/False/None (use 'is' or 'is not')",
                            self.code(),
                            context,
                            comparator.range().start(),
                            "LOW",
                        )]);
                    }
                }
            }
        }
        None
    }
}

struct ArgumentCountRule {
    max_args: usize,
}
impl ArgumentCountRule {
    fn new(max_args: usize) -> Self {
        Self { max_args }
    }
}
impl Rule for ArgumentCountRule {
    fn name(&self) -> &'static str {
        "ArgumentCountRule"
    }
    fn code(&self) -> &'static str {
        "CSP-C303"
    }
    fn enter_stmt(&mut self, stmt: &Stmt, context: &Context) -> Option<Vec<Finding>> {
        let parameters = match stmt {
            Stmt::FunctionDef(f) => &f.parameters,
            _ => return None,
        };

        let total_args = parameters.posonlyargs.len()
            + parameters.args.len()
            + parameters.kwonlyargs.len()
            + usize::from(parameters.vararg.is_some())
            + usize::from(parameters.kwarg.is_some());

        if total_args > self.max_args {
            return Some(vec![create_finding(
                &format!("Too many arguments ({} > {})", total_args, self.max_args),
                self.code(),
                context,
                stmt.range().start(),
                "LOW",
            )]);
        }
        None
    }
}

struct FunctionLengthRule {
    max_lines: usize,
}
impl FunctionLengthRule {
    fn new(max_lines: usize) -> Self {
        Self { max_lines }
    }
}
impl Rule for FunctionLengthRule {
    fn name(&self) -> &'static str {
        "FunctionLengthRule"
    }
    fn code(&self) -> &'static str {
        "CSP-C304"
    }
    fn enter_stmt(&mut self, stmt: &Stmt, context: &Context) -> Option<Vec<Finding>> {
        if let Stmt::FunctionDef(_) = stmt {
            let start_line = context.line_index.line_index(stmt.range().start());
            let end_line = context.line_index.line_index(stmt.range().end());
            let length = end_line - start_line + 1;

            if length > self.max_lines {
                return Some(vec![create_finding(
                    &format!("Function too long ({} > {} lines)", length, self.max_lines),
                    self.code(),
                    context,
                    stmt.range().start(),
                    "LOW",
                )]);
            }
        }
        None
    }
}

struct ComplexityRule {
    threshold: usize,
}
impl ComplexityRule {
    fn new(threshold: usize) -> Self {
        Self { threshold }
    }
}
impl Rule for ComplexityRule {
    fn name(&self) -> &'static str {
        "ComplexityRule"
    }
    fn code(&self) -> &'static str {
        "CSP-Q301"
    }
    fn enter_stmt(&mut self, stmt: &Stmt, context: &Context) -> Option<Vec<Finding>> {
        match stmt {
            Stmt::FunctionDef(f) => self.check_complexity(&f.body, stmt, context),
            _ => None,
        }
    }
}

impl ComplexityRule {
    fn check_complexity(
        &self,
        body: &[Stmt],
        stmt: &Stmt,
        context: &Context,
    ) -> Option<Vec<Finding>> {
        let complexity = 1 + calculate_complexity(body);
        if complexity > self.threshold {
            let severity = if complexity > 25 {
                "CRITICAL"
            } else if complexity > 15 {
                "HIGH"
            } else {
                "MEDIUM"
            };
            Some(vec![create_finding(
                &format!("Function is too complex (McCabe={complexity})"),
                self.code(),
                context,
                stmt.range().start(),
                severity,
            )])
        } else {
            None
        }
    }
}

fn calculate_complexity(stmts: &[Stmt]) -> usize {
    let mut complexity = 0;
    for stmt in stmts {
        complexity += match stmt {
            Stmt::If(n) => {
                let mut sum = 1 + calculate_complexity(&n.body);
                for clause in &n.elif_else_clauses {
                    // Only elif adds complexity, else doesn't
                    if clause.test.is_some() {
                        sum += 1;
                    }
                    sum += calculate_complexity(&clause.body);
                }
                sum
            }
            Stmt::For(n) => 1 + calculate_complexity(&n.body) + calculate_complexity(&n.orelse),
            Stmt::While(n) => 1 + calculate_complexity(&n.body) + calculate_complexity(&n.orelse),
            Stmt::Try(n) => {
                n.handlers.len()
                    + calculate_complexity(&n.body)
                    + calculate_complexity(&n.orelse)
                    + calculate_complexity(&n.finalbody)
            }
            Stmt::With(n) => calculate_complexity(&n.body),
            _ => 0, // Don't recurse into nested functions/classes for this function's complexity
        };
    }
    complexity
}

struct NestingRule {
    current_depth: usize,
    max_depth: usize,
    /// Track lines we've already reported to avoid duplicates
    reported_lines: std::collections::HashSet<usize>,
}

impl NestingRule {
    fn new(max_depth: usize) -> Self {
        Self {
            current_depth: 0,
            max_depth,
            reported_lines: std::collections::HashSet::new(),
        }
    }

    fn check_depth(
        &mut self,
        context: &Context,
        location: ruff_text_size::TextSize,
    ) -> Option<Finding> {
        if self.current_depth > self.max_depth {
            let line = context.line_index.line_index(location);
            // Only report once per line
            if self.reported_lines.contains(&line) {
                return None;
            }
            self.reported_lines.insert(line);
            Some(Finding {
                message: format!("Deeply nested code (depth {})", self.current_depth),
                rule_id: self.code().to_owned(),
                file: context.filename.clone(),
                line,
                col: 0,
                severity: "LOW".to_owned(),
            })
        } else {
            None
        }
    }

    fn should_increase_depth(stmt: &Stmt) -> bool {
        matches!(
            stmt,
            Stmt::FunctionDef(_)
                | Stmt::ClassDef(_)
                | Stmt::If(_)
                | Stmt::For(_)
                | Stmt::While(_)
                | Stmt::Try(_)
                | Stmt::With(_)
        )
    }
}

impl Rule for NestingRule {
    fn name(&self) -> &'static str {
        "NestingRule"
    }
    fn code(&self) -> &'static str {
        "CSP-Q302"
    }

    fn enter_stmt(&mut self, stmt: &Stmt, context: &Context) -> Option<Vec<Finding>> {
        if Self::should_increase_depth(stmt) {
            self.current_depth += 1;

            if let Some(f) = self.check_depth(context, stmt.range().start()) {
                return Some(vec![f]);
            }
        }
        None
    }

    fn leave_stmt(&mut self, stmt: &Stmt, _context: &Context) -> Option<Vec<Finding>> {
        if Self::should_increase_depth(stmt) && self.current_depth > 0 {
            self.current_depth -= 1;
        }
        None
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
