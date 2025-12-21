use crate::metrics::cc_rank;
use crate::utils::LineIndex;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

#[derive(Debug, Clone, PartialEq)]
/// A finding related to Cyclomatic Complexity.
pub struct ComplexityFinding {
    /// Name of the function, class, or method.
    pub name: String,
    /// The calculated cyclomatic complexity score.
    pub complexity: usize,
    /// The complexity rank (A-F).
    pub rank: char,
    /// The type of the block ("function", "method", "class").
    pub type_: String,
    /// The line number where the block starts.
    pub line: usize,
}

/// Analyzes the cyclomatic complexity of code within a file.
///
/// # Arguments
/// * `code` - The source code to analyze
/// * `path` - The file path (for error messages)
/// * `no_assert` - If true, assert statements don't add to complexity
#[must_use]
pub fn analyze_complexity(
    code: &str,
    _path: &std::path::Path,
    no_assert: bool,
) -> Vec<ComplexityFinding> {
    let mut findings = Vec::new();
    if let Ok(parsed) = ruff_python_parser::parse_module(code) {
        let module = parsed.into_syntax();
        let line_index = LineIndex::new(code);
        let mut visitor = ComplexityVisitor {
            findings: Vec::new(),
            line_index: &line_index,
            class_stack: Vec::new(),
            no_assert,
        };
        visitor.visit_body(&module.body);
        findings = visitor.findings;
    }
    findings
}

/// Calculates the total cyclomatic complexity of a module (sum of all blocks).
/// Note: Uses `no_assert=false` as this is typically used for MI calculation.
#[must_use]
pub fn calculate_module_complexity(code: &str) -> Option<usize> {
    if let Ok(parsed) = ruff_python_parser::parse_module(code) {
        let module = parsed.into_syntax();
        return Some(calculate_complexity(&module.body, false));
    }
    None
}

struct ComplexityVisitor<'a> {
    findings: Vec<ComplexityFinding>,
    line_index: &'a LineIndex,
    class_stack: Vec<String>,
    no_assert: bool,
}

impl ComplexityVisitor<'_> {
    fn visit_body(&mut self, body: &[Stmt]) {
        for stmt in body {
            self.visit_stmt(stmt);
        }
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::FunctionDef(node) => {
                let complexity = calculate_complexity(&node.body, self.no_assert);
                let rank = cc_rank(complexity);
                let line = self.line_index.line_index(node.start());
                let type_ = if self.class_stack.is_empty() {
                    "function"
                } else {
                    "method"
                };

                self.findings.push(ComplexityFinding {
                    name: node.name.to_string(),
                    complexity,
                    rank,
                    type_: type_.to_owned(),
                    line,
                });

                // Recurse to find nested blocks
                self.visit_body(&node.body);
            }
            Stmt::ClassDef(node) => {
                let complexity = calculate_complexity(&node.body, self.no_assert);
                let rank = cc_rank(complexity);
                let line = self.line_index.line_index(node.start());

                self.findings.push(ComplexityFinding {
                    name: node.name.to_string(),
                    complexity,
                    rank,
                    type_: "class".to_owned(),
                    line,
                });

                self.class_stack.push(node.name.to_string());
                self.visit_body(&node.body);
                self.class_stack.pop();
            }
            _ => {
                // For other statements, we might need to recurse if they contain blocks (e.g. if/for/while)
                // BUT `calculate_complexity` handles the complexity of the *current* block.
                // We only want to find *nested definitions* here.
                // So we need to traverse into If/For/While/Try/With bodies to find nested functions/classes.
                match stmt {
                    Stmt::If(node) => {
                        self.visit_body(&node.body);
                        for clause in &node.elif_else_clauses {
                            self.visit_body(&clause.body);
                        }
                    }
                    Stmt::For(node) => {
                        self.visit_body(&node.body);
                        self.visit_body(&node.orelse);
                    }
                    Stmt::While(node) => {
                        self.visit_body(&node.body);
                        self.visit_body(&node.orelse);
                    }

                    Stmt::With(node) => {
                        self.visit_body(&node.body);
                    }
                    Stmt::Try(node) => {
                        self.visit_body(&node.body);
                        for handler in &node.handlers {
                            let ast::ExceptHandler::ExceptHandler(h) = handler;
                            self.visit_body(&h.body);
                        }
                        self.visit_body(&node.finalbody);
                        self.visit_body(&node.orelse);
                    }
                    Stmt::Match(node) => {
                        for case in &node.cases {
                            self.visit_body(&case.body);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn calculate_complexity(body: &[Stmt], no_assert: bool) -> usize {
    let mut visitor = BlockComplexityVisitor {
        complexity: 1,
        no_assert,
    };
    visitor.visit_body(body);
    visitor.complexity
}

struct BlockComplexityVisitor {
    complexity: usize,
    no_assert: bool,
}

impl BlockComplexityVisitor {
    fn visit_body(&mut self, body: &[Stmt]) {
        for stmt in body {
            self.visit_stmt(stmt);
        }
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::If(node) => {
                self.complexity += 1;
                self.visit_expr(&node.test);
                self.visit_body(&node.body);
                for clause in &node.elif_else_clauses {
                    // Only elif adds complexity, else doesn't
                    if let Some(test) = &clause.test {
                        self.complexity += 1;
                        self.visit_expr(test);
                    }
                    self.visit_body(&clause.body);
                }
            }
            Stmt::For(node) => {
                self.complexity += 1;
                self.visit_expr(&node.target);
                self.visit_expr(&node.iter);
                self.visit_body(&node.body);
                self.visit_body(&node.orelse);
            }
            Stmt::While(node) => {
                self.complexity += 1;
                self.visit_expr(&node.test);
                self.visit_body(&node.body);
                self.visit_body(&node.orelse);
            }
            Stmt::Try(node) => {
                self.visit_body(&node.body);
                for handler in &node.handlers {
                    self.complexity += 1; // except block adds 1
                    let ast::ExceptHandler::ExceptHandler(h) = handler;
                    if let Some(type_) = &h.type_ {
                        self.visit_expr(type_);
                    }
                    self.visit_body(&h.body);
                }
                self.visit_body(&node.orelse);
                self.visit_body(&node.finalbody);
            }
            Stmt::With(node) => {
                for item in &node.items {
                    self.visit_expr(&item.context_expr);
                    if let Some(optional_vars) = &item.optional_vars {
                        self.visit_expr(optional_vars);
                    }
                }
                self.visit_body(&node.body);
            }
            Stmt::Assert(node) => {
                // Only add complexity for assert if no_assert is false
                if !self.no_assert {
                    self.complexity += 1;
                }
                self.visit_expr(&node.test);
                if let Some(msg) = &node.msg {
                    self.visit_expr(msg);
                }
            }
            Stmt::Match(node) => {
                self.visit_expr(&node.subject);
                for case in &node.cases {
                    self.complexity += 1;
                    if let Some(guard) = &case.guard {
                        self.visit_expr(guard);
                    }
                    self.visit_body(&case.body);
                }
            }
            // Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {} // Do NOT recurse for block complexity
            Stmt::Expr(node) => {
                self.visit_expr(&node.value);
            }
            Stmt::Return(node) => {
                if let Some(value) = &node.value {
                    self.visit_expr(value);
                }
            }
            Stmt::Assign(node) => {
                for target in &node.targets {
                    self.visit_expr(target);
                }
                self.visit_expr(&node.value);
            }
            Stmt::AugAssign(node) => {
                self.visit_expr(&node.target);
                self.visit_expr(&node.value);
            }
            Stmt::AnnAssign(node) => {
                self.visit_expr(&node.target);
                self.visit_expr(&node.annotation);
                if let Some(value) = &node.value {
                    self.visit_expr(value);
                }
            }
            Stmt::Delete(node) => {
                for target in &node.targets {
                    self.visit_expr(target);
                }
            }
            Stmt::Raise(node) => {
                if let Some(exc) = &node.exc {
                    self.visit_expr(exc);
                }
                if let Some(cause) = &node.cause {
                    self.visit_expr(cause);
                }
            }
            _ => {}
        }
    }

    fn visit_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::BoolOp(node) => {
                // Each boolean operator adds 1 (short-circuiting)
                // `a and b` -> 1 op -> +1 complexity?
                // `a and b and c` -> 2 ops -> +2 complexity?
                // `node.values` has N items. Ops = N - 1.
                if node.values.len() > 1 {
                    self.complexity += node.values.len() - 1;
                }
                for value in &node.values {
                    self.visit_expr(value);
                }
            }
            Expr::If(node) => {
                self.complexity += 1;
                self.visit_expr(&node.test);
                self.visit_expr(&node.body);
                self.visit_expr(&node.orelse);
            }
            Expr::ListComp(node) => {
                self.complexity += node.generators.len(); // Each generator is a loop
                for gen in &node.generators {
                    self.complexity += gen.ifs.len(); // Each if filter is a branch
                    self.visit_expr(&gen.target);
                    self.visit_expr(&gen.iter);
                    for if_ in &gen.ifs {
                        self.visit_expr(if_);
                    }
                }
                self.visit_expr(&node.elt);
            }
            Expr::SetComp(node) => {
                self.complexity += node.generators.len();
                for gen in &node.generators {
                    self.complexity += gen.ifs.len();
                    self.visit_expr(&gen.target);
                    self.visit_expr(&gen.iter);
                    for if_ in &gen.ifs {
                        self.visit_expr(if_);
                    }
                }
                self.visit_expr(&node.elt);
            }
            Expr::DictComp(node) => {
                self.complexity += node.generators.len();
                for gen in &node.generators {
                    self.complexity += gen.ifs.len();
                    self.visit_expr(&gen.target);
                    self.visit_expr(&gen.iter);
                    for if_ in &gen.ifs {
                        self.visit_expr(if_);
                    }
                }
                self.visit_expr(&node.key);
                self.visit_expr(&node.value);
            }
            Expr::Generator(node) => {
                self.complexity += node.generators.len();
                for gen in &node.generators {
                    self.complexity += gen.ifs.len();
                    self.visit_expr(&gen.target);
                    self.visit_expr(&gen.iter);
                    for if_ in &gen.ifs {
                        self.visit_expr(if_);
                    }
                }
                self.visit_expr(&node.elt);
            }
            Expr::Lambda(_node) => {
                // Lambda is a function, does it add to current complexity?
                // Radon: "Lambdas are functions, so they have their own complexity."
                // But does the *enclosing* function get +1?
                // No.
                // But we should probably visit lambda body?
                // If we visit lambda body, we might count branches inside lambda towards enclosing function?
                // That would be wrong if lambda is separate block.
                // So we should NOT visit lambda body for *this* block's complexity.
            }
            // Recurse for other expressions
            Expr::Call(node) => {
                self.visit_expr(&node.func);
                for arg in &node.arguments.args {
                    self.visit_expr(arg);
                }
                for kw in &node.arguments.keywords {
                    self.visit_expr(&kw.value);
                }
            }
            // ... add other expressions if needed ...
            _ => {}
        }
    }
}
