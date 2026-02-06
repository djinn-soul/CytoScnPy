use super::{finding::create_finding, CAT_MAINTAINABILITY};
use crate::metrics::cognitive_complexity::calculate_cognitive_complexity;
use crate::metrics::lcom4::calculate_lcom4;
use crate::rules::ids;
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextSize};
const META_COMPLEXITY: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_COMPLEXITY,
    category: CAT_MAINTAINABILITY,
};
const META_COGNITIVE_COMPLEXITY: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_COGNITIVE_COMPLEXITY,
    category: CAT_MAINTAINABILITY,
};
const META_COHESION: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_COHESION,
    category: CAT_MAINTAINABILITY,
};
pub(super) struct ComplexityRule {
    threshold: usize,
}
impl ComplexityRule {
    pub(super) fn new(threshold: usize) -> Self {
        Self { threshold }
    }
    fn check_complexity(
        &self,
        body: &[Stmt],
        name_start: TextSize,
        context: &Context,
    ) -> Option<Vec<Finding>> {
        let complexity = calculate_function_complexity(body);
        if complexity <= self.threshold {
            return None;
        }
        let severity = if complexity > 25 {
            "CRITICAL"
        } else if complexity > 15 {
            "HIGH"
        } else {
            "MEDIUM"
        };
        Some(vec![create_finding(
            &format!("Function is too complex (McCabe={complexity})"),
            META_COMPLEXITY,
            context,
            name_start,
            severity,
        )])
    }
}
impl Rule for ComplexityRule {
    fn name(&self) -> &'static str {
        "ComplexityRule"
    }
    fn metadata(&self) -> RuleMetadata {
        META_COMPLEXITY
    }
    fn enter_stmt(&mut self, stmt: &Stmt, context: &Context) -> Option<Vec<Finding>> {
        match stmt {
            Stmt::FunctionDef(f) => self.check_complexity(&f.body, f.name.range().start(), context),
            _ => None,
        }
    }
}
fn calculate_function_complexity(stmts: &[Stmt]) -> usize {
    1 + calculate_complexity(stmts)
}
fn calculate_complexity(stmts: &[Stmt]) -> usize {
    let mut complexity = 0;
    for stmt in stmts {
        complexity += match stmt {
            Stmt::If(n) => {
                let mut sum = 1 + calculate_complexity(&n.body);
                for clause in &n.elif_else_clauses {
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
            Stmt::Match(n) => {
                let mut sum = 1;
                for case in &n.cases {
                    sum += calculate_complexity(&case.body);
                }
                sum
            }
            _ => 0,
        };
    }
    complexity
}
pub(super) struct CognitiveComplexityRule {
    threshold: usize,
}
impl CognitiveComplexityRule {
    pub(super) fn new(threshold: usize) -> Self {
        Self { threshold }
    }
}
impl Rule for CognitiveComplexityRule {
    fn name(&self) -> &'static str {
        "CognitiveComplexityRule"
    }
    fn metadata(&self) -> RuleMetadata {
        META_COGNITIVE_COMPLEXITY
    }
    fn enter_stmt(&mut self, stmt: &Stmt, context: &Context) -> Option<Vec<Finding>> {
        let (body, name_start) = match stmt {
            Stmt::FunctionDef(f) => (&f.body, f.name.range().start()),
            _ => return None,
        };
        let complexity = calculate_cognitive_complexity(body);
        if complexity <= self.threshold {
            return None;
        }
        let severity = if complexity > 25 { "CRITICAL" } else { "HIGH" };
        Some(vec![create_finding(
            &format!(
                "Cognitive Complexity is too high ({complexity} > {})",
                self.threshold
            ),
            META_COGNITIVE_COMPLEXITY,
            context,
            name_start,
            severity,
        )])
    }
}
pub(super) struct CohesionRule {
    threshold: usize,
    pydantic_imported: bool,
}
impl CohesionRule {
    pub(super) fn new(threshold: usize) -> Self {
        Self {
            threshold,
            pydantic_imported: false,
        }
    }

    fn update_import_state(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Import(node) => {
                for alias in &node.names {
                    if alias.name.as_str().contains("pydantic") {
                        self.pydantic_imported = true;
                        break;
                    }
                }
            }
            Stmt::ImportFrom(node) => {
                if let Some(module) = &node.module {
                    let base = module.split('.').next().unwrap_or("");
                    if base == "pydantic" {
                        self.pydantic_imported = true;
                    }
                }
            }
            _ => {}
        }
    }

    fn is_exempt_class(&self, class_def: &ast::StmtClassDef) -> bool {
        if class_def
            .decorator_list
            .iter()
            .any(|d| is_dataclass_decorator(&d.expression) || is_attrs_decorator(&d.expression))
        {
            return true;
        }

        class_def.bases().iter().any(|base| {
            is_protocol_base(base)
                || is_typing_model_base(base)
                || is_pydantic_base(base, self.pydantic_imported)
        })
    }
}
impl Rule for CohesionRule {
    fn name(&self) -> &'static str {
        "CohesionRule"
    }
    fn metadata(&self) -> RuleMetadata {
        META_COHESION
    }
    fn enter_stmt(&mut self, stmt: &Stmt, context: &Context) -> Option<Vec<Finding>> {
        self.update_import_state(stmt);
        if let Stmt::ClassDef(c) = stmt {
            if self.is_exempt_class(c) {
                return None;
            }
            let lcom4 = calculate_lcom4(&c.body);
            if lcom4 > self.threshold {
                return Some(vec![create_finding(
                    &format!(
                        "Class has low cohesion (LCOM4={lcom4}). LCOM4=1 is cohesive; {lcom4} means {lcom4} disconnected method groups. Consider splitting or sharing state."
                    ),
                    META_COHESION,
                    context,
                    c.name.range().start(),
                    "HIGH",
                )]);
            }
        }
        None
    }
}

fn is_dataclass_decorator(expr: &Expr) -> bool {
    match expr {
        Expr::Name(name) => name.id == "dataclass",
        Expr::Attribute(attr) => attr.attr.as_str() == "dataclass",
        Expr::Call(call) => is_dataclass_decorator(&call.func),
        _ => false,
    }
}

fn is_attrs_decorator(expr: &Expr) -> bool {
    match expr {
        Expr::Attribute(attr) => {
            let base_is_attr = match &*attr.value {
                Expr::Name(name) => name.id == "attr" || name.id == "attrs",
                Expr::Attribute(inner) => {
                    inner.attr.as_str() == "attr" || inner.attr.as_str() == "attrs"
                }
                _ => false,
            };
            base_is_attr && matches!(attr.attr.as_str(), "s" | "define" | "frozen" | "mutable")
        }
        Expr::Call(call) => is_attrs_decorator(&call.func),
        _ => false,
    }
}

fn is_protocol_base(expr: &Expr) -> bool {
    match expr {
        Expr::Name(name) => name.id == "Protocol",
        Expr::Attribute(attr) => attr.attr.as_str() == "Protocol",
        _ => false,
    }
}

fn is_typing_model_base(expr: &Expr) -> bool {
    match expr {
        Expr::Name(name) => matches!(name.id.as_str(), "TypedDict" | "NamedTuple" | "Struct"),
        Expr::Attribute(attr) => {
            matches!(attr.attr.as_str(), "TypedDict" | "NamedTuple" | "Struct")
        }
        _ => false,
    }
}

fn is_pydantic_base(expr: &Expr, pydantic_imported: bool) -> bool {
    match expr {
        Expr::Name(name) => pydantic_imported && name.id == "BaseModel",
        Expr::Attribute(attr) => {
            if attr.attr.as_str() != "BaseModel" {
                return false;
            }
            expr_contains_name(&attr.value, "pydantic")
        }
        _ => false,
    }
}

fn expr_contains_name(expr: &Expr, target: &str) -> bool {
    match expr {
        Expr::Name(name) => name.id == target,
        Expr::Attribute(attr) => {
            attr.attr.as_str() == target || expr_contains_name(&attr.value, target)
        }
        _ => false,
    }
}
