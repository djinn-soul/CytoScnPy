use super::{finding::create_finding, CAT_MAINTAINABILITY};
use crate::metrics::cognitive_complexity::calculate_cognitive_complexity;
use crate::metrics::lcom4::calculate_lcom4;
use crate::rules::ids;
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::Stmt;
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
}
impl CohesionRule {
    pub(super) fn new(threshold: usize) -> Self {
        Self { threshold }
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
        if let Stmt::ClassDef(c) = stmt {
            let lcom4 = calculate_lcom4(&c.body);
            if lcom4 > self.threshold {
                return Some(vec![create_finding(
                    &format!(
                        "Class lacks cohesion (LCOM4={lcom4}). Consider splitting into {lcom4} classes."
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
