use ruff_python_ast::{self as ast, Expr, Stmt};

/// Calculates the Cognitive Complexity of a statement (usually a `FunctionDef` body).
///
/// Based on the `SonarSource` Cognitive Complexity whitepaper.
///
/// # Returns
/// The integer complexity score.
pub fn calculate_cognitive_complexity(stmts: &[Stmt]) -> usize {
    let mut visitor = CognitiveComplexityVisitor::new();
    for stmt in stmts {
        visitor.visit_stmt(stmt);
    }
    visitor.complexity
}

struct CognitiveComplexityVisitor {
    complexity: usize,
    nesting_level: usize,
}

impl CognitiveComplexityVisitor {
    fn new() -> Self {
        Self {
            complexity: 0,
            nesting_level: 0,
        }
    }

    fn increase_nesting(&mut self) {
        self.nesting_level += 1;
    }

    fn decrease_nesting(&mut self) {
        if self.nesting_level > 0 {
            self.nesting_level -= 1;
        }
    }

    fn increment(&mut self, nesting: bool) {
        self.complexity += 1;
        if nesting {
            self.complexity += self.nesting_level;
        }
    }

    // visit_params was removed as it was never used.
    // Parameter defaults can contain complex expressions, but we ignore them
    // to match typical cognitive complexity implementations.

    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::If(ast::StmtIf {
                test,
                body,
                elif_else_clauses,
                range: _,
                ..
            }) => {
                self.increment(true);
                self.visit_expr(test);
                self.increase_nesting();
                for s in body {
                    self.visit_stmt(s);
                }
                self.decrease_nesting();

                for clause in elif_else_clauses {
                    self.visit_elif_else_clause(clause);
                }
            }
            Stmt::For(node) => {
                self.increment(true);
                self.visit_expr(&node.iter);
                self.increase_nesting();
                for s in &node.body {
                    self.visit_stmt(s);
                }
                self.decrease_nesting();
                for s in &node.orelse {
                    self.visit_stmt(s);
                }
            }
            Stmt::While(node) => {
                self.increment(true);
                self.visit_expr(&node.test);
                self.increase_nesting();
                for s in &node.body {
                    self.visit_stmt(s);
                }
                self.decrease_nesting();
                for s in &node.orelse {
                    self.visit_stmt(s);
                }
            }
            Stmt::Try(node) => {
                // Try itself doesn't increment, but catch (except) does
                for s in &node.body {
                    self.visit_stmt(s);
                }
                for handler in &node.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = handler;
                    self.increment(true);
                    self.increase_nesting();
                    for s in &h.body {
                        self.visit_stmt(s);
                    }
                    self.decrease_nesting();
                }
                for s in &node.orelse {
                    self.visit_stmt(s);
                }
                for s in &node.finalbody {
                    self.visit_stmt(s);
                }
            }
            Stmt::With(node) => {
                // 'with' itself usually doesn't increase complexity in standard CC,
                // but nested structures inside it do.
                // Sonar: "With" does NOT increment.
                // However, we must recurse.
                // We do NOT increase nesting level for 'with' in standard CC.
                for s in &node.body {
                    self.visit_stmt(s);
                }
            }
            Stmt::FunctionDef(node) => {
                // Nested function
                self.increase_nesting();
                self.complexity += self.nesting_level + 1; // +1 for the function itself + nesting
                for s in &node.body {
                    self.visit_stmt(s);
                }
                self.decrease_nesting();
            }
            Stmt::ClassDef(node) => {
                // Nested class
                // We don't usually punish class definitions themselves, but we recurse.
                // We do NOT increase nesting level for class bodies generally in flow metrics,
                // but let's just visit body.
                for s in &node.body {
                    self.visit_stmt(s);
                }
            }
            Stmt::Match(node) => {
                // Structural Pattern Matching (match/case)
                // Treated similar to switch
                self.visit_expr(&node.subject);
                for case in &node.cases {
                    self.increment(true);
                    self.increase_nesting();
                    for s in &case.body {
                        self.visit_stmt(s);
                    }
                    self.decrease_nesting();
                }
            }
            // Basic recursion for other statements
            // Assign, Expr, etc.
            Stmt::Expr(node) => self.visit_expr(&node.value),
            Stmt::Assign(node) => self.visit_expr(&node.value),
            Stmt::AnnAssign(node) => {
                if let Some(val) = &node.value {
                    self.visit_expr(val);
                }
            }
            Stmt::Return(node) => {
                if let Some(val) = &node.value {
                    self.visit_expr(val);
                }
            }
            _ => {
                // Default recursion handled if needed, for flat lists like Import, checking children isn't needed for score
            }
        }
    }

    fn visit_elif_else_clause(&mut self, clause: &ast::ElifElseClause) {
        if clause.test.is_some() {
            // 'else if' -> Increment
            self.increment(true);
            if let Some(test) = clause.test.as_ref() {
                self.visit_expr(test);
            }
            self.increase_nesting();
            for s in &clause.body {
                self.visit_stmt(s);
            }
            self.decrease_nesting();
        } else {
            // 'else' -> No increment, but check nesting
            // Sonar says 'else' does NOT increment complexity, but code INSIDE it receives nesting penalty?
            // Actually, 'else' statement itself does not increment, but the structural nesting usually applies.
            // However, strictly: "The else, default and finally structures do not increment the Nesting level."
            // So we just visit body.
            for s in &clause.body {
                self.visit_stmt(s);
            }
        }
    }

    fn visit_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::BoolOp(node) => {
                // Count sequential boolean operators.
                // A && B && C -> +1 for the sequence
                // A && B || C -> +1 for &&, +1 for ||
                // Currently, BoolOp groups same ops.
                // If mixed, it's nested BoolOps.
                // So we imply +1 for this BoolOp group.
                self.complexity += 1;

                for val in &node.values {
                    self.visit_expr(val);
                }
            }
            Expr::Lambda(node) => {
                // Treat lambda like function
                self.increase_nesting();
                self.complexity += self.nesting_level + 1;
                self.visit_expr(&node.body);
                self.decrease_nesting();
            }
            Expr::If(node) => {
                // Ternary: +1
                self.increment(false); // Ternaries don't usually nest structurally in the same way, but let's stick to simple +1
                self.visit_expr(&node.test);
                self.visit_expr(&node.body);
                self.visit_expr(&node.orelse);
            }
            _ => {
                // Recurse if needed (e.g. Call arguments)
                // Keeping it shallow for performance unless deep inspection needed
            }
        }
    }
}
