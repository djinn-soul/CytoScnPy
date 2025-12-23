use ruff_python_ast::{self as ast, Expr, Stmt};
use rustc_hash::FxHashSet;

use serde::Serialize;

#[derive(Debug, Default, Clone, PartialEq, Serialize)]
/// Metrics calculated using Halstead's Complexity Measures.
pub struct HalsteadMetrics {
    /// N1: Total number of operators.
    pub h1: usize,
    /// N2: Total number of operands.
    pub h2: usize,
    /// n1: Number of distinct operators.
    pub n1: usize,
    /// n2: Number of distinct operands.
    pub n2: usize,
    /// Halstead Program Vocabulary (n1 + n2).
    pub vocabulary: f64,
    /// Halstead Program Length (N1 + N2).
    pub length: f64,
    /// Calculated Program Length (n1 * log2(n1) + n2 * log2(n2)).
    pub calculated_length: f64,
    /// Halstead Volume (Length * log2(Vocabulary)).
    pub volume: f64,
    /// Halstead Difficulty ((n1 / 2) * (N2 / n2)).
    pub difficulty: f64,
    /// Halstead Effort (Difficulty * Volume).
    pub effort: f64,
    /// Estimated implementation time (Effort / 18).
    pub time: f64,
    /// Estimated number of delivered bugs (Volume / 3000).
    pub bugs: f64,
}

/// Calculates Halstead metrics for a given AST module.
pub fn analyze_halstead(ast: &ast::Mod) -> HalsteadMetrics {
    let mut visitor = HalsteadVisitor::new();
    visitor.visit_mod(ast);
    visitor.calculate_metrics()
}

/// Calculates Halstead metrics for each function in a given AST module.
pub fn analyze_halstead_functions(ast: &ast::Mod) -> Vec<(String, HalsteadMetrics)> {
    let mut visitor = FunctionHalsteadVisitor::new();
    visitor.visit_mod(ast);
    visitor.results
}

struct FunctionHalsteadVisitor {
    results: Vec<(String, HalsteadMetrics)>,
}

impl FunctionHalsteadVisitor {
    fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    fn visit_mod(&mut self, module: &ast::Mod) {
        if let ast::Mod::Module(m) = module {
            for stmt in &m.body {
                self.visit_stmt(stmt);
            }
        }
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::FunctionDef(node) => {
                let mut visitor = HalsteadVisitor::new();
                if node.is_async {
                    visitor.add_operator("async def");
                }
                // Visit function body
                for s in &node.body {
                    visitor.visit_stmt(s);
                }
                // Also visit arguments
                for arg in &node.parameters.args {
                    visitor.add_operand(&arg.parameter.name);
                }
                // posonlyargs
                for arg in &node.parameters.posonlyargs {
                    visitor.add_operand(&arg.parameter.name);
                }
                // kwonlyargs
                for arg in &node.parameters.kwonlyargs {
                    visitor.add_operand(&arg.parameter.name);
                }
                self.results
                    .push((node.name.to_string(), visitor.calculate_metrics()));

                // Recurse for nested functions
                for s in &node.body {
                    self.visit_stmt(s);
                }
            }
            Stmt::ClassDef(node) => {
                for s in &node.body {
                    self.visit_stmt(s);
                }
            }
            _ => {
                // For other statements, we might need to recurse if they contain blocks
                // But we are only looking for function definitions
                match stmt {
                    Stmt::If(node) => {
                        for s in &node.body {
                            self.visit_stmt(s);
                        }
                        for clause in &node.elif_else_clauses {
                            self.visit_stmt(&clause.body[0]); // Approximation or iterate body
                                                              // elif_else_clauses contains ElifElseClause which has `body` (Vec<Stmt>)
                            for s in &clause.body {
                                self.visit_stmt(s);
                            }
                        }
                    }
                    Stmt::For(node) => {
                        for s in &node.body {
                            self.visit_stmt(s);
                        }
                        for s in &node.orelse {
                            self.visit_stmt(s);
                        }
                    }
                    Stmt::While(node) => {
                        for s in &node.body {
                            self.visit_stmt(s);
                        }
                        for s in &node.orelse {
                            self.visit_stmt(s);
                        }
                    }
                    Stmt::With(node) => {
                        for s in &node.body {
                            self.visit_stmt(s);
                        }
                    }
                    Stmt::Try(node) => {
                        for s in &node.body {
                            self.visit_stmt(s);
                        }
                        for handler in &node.handlers {
                            let ast::ExceptHandler::ExceptHandler(h) = handler;
                            for s in &h.body {
                                self.visit_stmt(s);
                            }
                        }
                        for s in &node.orelse {
                            self.visit_stmt(s);
                        }
                        for s in &node.finalbody {
                            self.visit_stmt(s);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

struct HalsteadVisitor {
    operators: FxHashSet<String>,
    operands: FxHashSet<String>,
    total_operators: usize,
    total_operands: usize,
}

impl HalsteadVisitor {
    fn new() -> Self {
        Self {
            operators: FxHashSet::default(),
            operands: FxHashSet::default(),
            total_operators: 0,
            total_operands: 0,
        }
    }

    fn add_operator(&mut self, op: &str) {
        self.operators.insert(op.to_owned());
        self.total_operators += 1;
    }

    fn add_operand(&mut self, op: &str) {
        self.operands.insert(op.to_owned());
        self.total_operands += 1;
    }

    #[allow(clippy::cast_precision_loss)]
    fn calculate_metrics(&self) -> HalsteadMetrics {
        let n1 = self.operators.len() as f64;
        let n2 = self.operands.len() as f64;
        let n1_total = self.total_operators as f64;
        let n2_total = self.total_operands as f64;

        let vocabulary = n1 + n2;
        let length = n1_total + n2_total;
        let calculated_length = if n1 > 0.0 && n2 > 0.0 {
            n1 * n1.log2() + n2 * n2.log2()
        } else {
            0.0
        };
        let volume = if vocabulary > 0.0 {
            length * vocabulary.log2()
        } else {
            0.0
        };
        let difficulty = if n2 > 0.0 {
            (n1 / 2.0) * (n2_total / n2)
        } else {
            0.0
        };
        let effort = difficulty * volume;
        let time = effort / 18.0;
        let bugs = volume / 3000.0;

        HalsteadMetrics {
            h1: self.total_operators,
            h2: self.total_operands,
            n1: self.operators.len(),
            n2: self.operands.len(),
            vocabulary,
            length,
            calculated_length,
            volume,
            difficulty,
            effort,
            time,
            bugs,
        }
    }

    fn visit_mod(&mut self, module: &ast::Mod) {
        if let ast::Mod::Module(m) = module {
            for stmt in &m.body {
                self.visit_stmt(stmt);
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::FunctionDef(node) => {
                if node.is_async {
                    self.add_operator("async def");
                } else {
                    self.add_operator("def");
                }
                self.add_operand(&node.name);
                for arg in &node.parameters.args {
                    self.add_operand(&arg.parameter.name);
                }
                for stmt in &node.body {
                    self.visit_stmt(stmt);
                }
            }
            Stmt::ClassDef(node) => {
                self.add_operator("class");
                self.add_operand(&node.name);
                for stmt in &node.body {
                    self.visit_stmt(stmt);
                }
            }
            Stmt::Return(node) => {
                self.add_operator("return");
                if let Some(value) = &node.value {
                    self.visit_expr(value);
                }
            }
            Stmt::Delete(node) => {
                self.add_operator("del");
                for target in &node.targets {
                    self.visit_expr(target);
                }
            }
            Stmt::Assign(node) => {
                self.add_operator("=");
                for target in &node.targets {
                    self.visit_expr(target);
                }
                self.visit_expr(&node.value);
            }
            Stmt::AugAssign(node) => {
                self.add_operator(match node.op {
                    ast::Operator::Add => "+=",
                    ast::Operator::Sub => "-=",
                    ast::Operator::Mult => "*=",
                    ast::Operator::MatMult => "@=",
                    ast::Operator::Div => "/=",
                    ast::Operator::Mod => "%=",
                    ast::Operator::Pow => "**=",
                    ast::Operator::LShift => "<<=",
                    ast::Operator::RShift => ">>=",
                    ast::Operator::BitOr => "|=",
                    ast::Operator::BitXor => "^=",
                    ast::Operator::BitAnd => "&=",
                    ast::Operator::FloorDiv => "//=",
                });
                self.visit_expr(&node.target);
                self.visit_expr(&node.value);
            }
            Stmt::AnnAssign(node) => {
                self.add_operator(":");
                self.add_operator("=");
                self.visit_expr(&node.target);
                if let Some(value) = &node.value {
                    self.visit_expr(value);
                }
            }
            Stmt::For(node) => {
                if node.is_async {
                    self.add_operator("async for");
                } else {
                    self.add_operator("for");
                }
                self.add_operator("in");
                self.visit_expr(&node.target);
                self.visit_expr(&node.iter);
                for stmt in &node.body {
                    self.visit_stmt(stmt);
                }
            }
            Stmt::While(node) => {
                self.add_operator("while");
                self.visit_expr(&node.test);
                for stmt in &node.body {
                    self.visit_stmt(stmt);
                }
            }
            Stmt::If(node) => {
                self.add_operator("if");
                self.visit_expr(&node.test);
                for stmt in &node.body {
                    self.visit_stmt(stmt);
                }
                for clause in &node.elif_else_clauses {
                    self.add_operator("else"); // counting elif as else + if usually, or just branches
                    for stmt in &clause.body {
                        self.visit_stmt(stmt);
                    }
                }
            }
            Stmt::With(node) => {
                if node.is_async {
                    self.add_operator("async with");
                } else {
                    self.add_operator("with");
                }
                for item in &node.items {
                    self.visit_expr(&item.context_expr);
                }
                for stmt in &node.body {
                    self.visit_stmt(stmt);
                }
            }
            Stmt::Raise(node) => {
                self.add_operator("raise");
                if let Some(exc) = &node.exc {
                    self.visit_expr(exc);
                }
                if let Some(cause) = &node.cause {
                    self.add_operator("from");
                    self.visit_expr(cause);
                }
            }
            Stmt::Try(node) => {
                self.add_operator("try");
                for stmt in &node.body {
                    self.visit_stmt(stmt);
                }
                for handler in &node.handlers {
                    self.add_operator("except");
                    let ast::ExceptHandler::ExceptHandler(h) = handler;
                    if let Some(type_) = &h.type_ {
                        self.visit_expr(type_);
                    }
                    for stmt in &h.body {
                        self.visit_stmt(stmt);
                    }
                }
                if !node.orelse.is_empty() {
                    self.add_operator("else");
                    for stmt in &node.orelse {
                        self.visit_stmt(stmt);
                    }
                }
                if !node.finalbody.is_empty() {
                    self.add_operator("finally");
                    for stmt in &node.finalbody {
                        self.visit_stmt(stmt);
                    }
                }
            }
            Stmt::Assert(node) => {
                self.add_operator("assert");
                self.visit_expr(&node.test);
                if let Some(msg) = &node.msg {
                    self.visit_expr(msg);
                }
            }
            Stmt::Import(node) => {
                self.add_operator("import");
                for alias in &node.names {
                    self.add_operand(&alias.name);
                    if let Some(asname) = &alias.asname {
                        self.add_operator("as");
                        self.add_operand(asname);
                    }
                }
            }
            Stmt::ImportFrom(node) => {
                self.add_operator("from");
                self.add_operator("import");
                if let Some(module) = &node.module {
                    self.add_operand(module);
                }
                for alias in &node.names {
                    self.add_operand(&alias.name);
                    if let Some(asname) = &alias.asname {
                        self.add_operator("as");
                        self.add_operand(asname);
                    }
                }
            }
            Stmt::Global(node) => {
                self.add_operator("global");
                for name in &node.names {
                    self.add_operand(name);
                }
            }
            Stmt::Nonlocal(node) => {
                self.add_operator("nonlocal");
                for name in &node.names {
                    self.add_operand(name);
                }
            }
            Stmt::Expr(node) => {
                self.visit_expr(&node.value);
            }
            Stmt::Pass(_) => {
                self.add_operator("pass");
            }
            Stmt::Break(_) => {
                self.add_operator("break");
            }
            Stmt::Continue(_) => {
                self.add_operator("continue");
            }
            _ => {}
        }
    }

    #[allow(clippy::too_many_lines)]
    fn visit_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::BoolOp(node) => {
                self.add_operator(match node.op {
                    ast::BoolOp::And => "and",
                    ast::BoolOp::Or => "or",
                });
                for value in &node.values {
                    self.visit_expr(value);
                }
            }
            Expr::Named(node) => {
                self.add_operator(":=");
                self.visit_expr(&node.target);
                self.visit_expr(&node.value);
            }
            Expr::BinOp(node) => {
                self.add_operator(match node.op {
                    ast::Operator::Add => "+",
                    ast::Operator::Sub => "-",
                    ast::Operator::Mult => "*",
                    ast::Operator::MatMult => "@",
                    ast::Operator::Div => "/",
                    ast::Operator::Mod => "%",
                    ast::Operator::Pow => "**",
                    ast::Operator::LShift => "<<",
                    ast::Operator::RShift => ">>",
                    ast::Operator::BitOr => "|",
                    ast::Operator::BitXor => "^",
                    ast::Operator::BitAnd => "&",
                    ast::Operator::FloorDiv => "//",
                });
                self.visit_expr(&node.left);
                self.visit_expr(&node.right);
            }
            Expr::UnaryOp(node) => {
                self.add_operator(match node.op {
                    ast::UnaryOp::Invert => "~",
                    ast::UnaryOp::Not => "not",
                    ast::UnaryOp::UAdd => "+",
                    ast::UnaryOp::USub => "-",
                });
                self.visit_expr(&node.operand);
            }
            Expr::Lambda(node) => {
                self.add_operator("lambda");
                if let Some(parameters) = &node.parameters {
                    for arg in &parameters.args {
                        self.add_operand(arg.parameter.name.as_str());
                    }
                }
                self.visit_expr(&node.body);
            }
            Expr::If(node) => {
                self.add_operator("if");
                self.add_operator("else");
                self.visit_expr(&node.test);
                self.visit_expr(&node.body);
                self.visit_expr(&node.orelse);
            }
            Expr::Dict(node) => {
                self.add_operator("{}");
                for item in &node.items {
                    if let Some(key) = &item.key {
                        self.visit_expr(key);
                    }
                    self.visit_expr(&item.value);
                }
            }
            Expr::Set(node) => {
                self.add_operator("{}");
                for elt in &node.elts {
                    self.visit_expr(elt);
                }
            }
            Expr::ListComp(node) => {
                self.add_operator("[]");
                self.visit_expr(&node.elt);
                for gen in &node.generators {
                    self.add_operator("for");
                    self.add_operator("in");
                    self.visit_expr(&gen.target);
                    self.visit_expr(&gen.iter);
                    for if_ in &gen.ifs {
                        self.add_operator("if");
                        self.visit_expr(if_);
                    }
                }
            }
            Expr::SetComp(node) => {
                self.add_operator("{}");
                self.visit_expr(&node.elt);
                for gen in &node.generators {
                    self.add_operator("for");
                    self.add_operator("in");
                    self.visit_expr(&gen.target);
                    self.visit_expr(&gen.iter);
                    for if_ in &gen.ifs {
                        self.add_operator("if");
                        self.visit_expr(if_);
                    }
                }
            }
            Expr::DictComp(node) => {
                self.add_operator("{}");
                self.visit_expr(&node.key);
                self.visit_expr(&node.value);
                for gen in &node.generators {
                    self.add_operator("for");
                    self.add_operator("in");
                    self.visit_expr(&gen.target);
                    self.visit_expr(&gen.iter);
                    for if_ in &gen.ifs {
                        self.add_operator("if");
                        self.visit_expr(if_);
                    }
                }
            }
            Expr::Generator(node) => {
                self.add_operator("()");
                self.visit_expr(&node.elt);
                for gen in &node.generators {
                    self.add_operator("for");
                    self.add_operator("in");
                    self.visit_expr(&gen.target);
                    self.visit_expr(&gen.iter);
                    for if_ in &gen.ifs {
                        self.add_operator("if");
                        self.visit_expr(if_);
                    }
                }
            }
            Expr::Await(node) => {
                self.add_operator("await");
                self.visit_expr(&node.value);
            }
            Expr::Yield(node) => {
                self.add_operator("yield");
                if let Some(value) = &node.value {
                    self.visit_expr(value);
                }
            }
            Expr::YieldFrom(node) => {
                self.add_operator("yield from");
                self.visit_expr(&node.value);
            }
            Expr::Compare(node) => {
                for op in &node.ops {
                    self.add_operator(match op {
                        ast::CmpOp::Eq => "==",
                        ast::CmpOp::NotEq => "!=",
                        ast::CmpOp::Lt => "<",
                        ast::CmpOp::LtE => "<=",
                        ast::CmpOp::Gt => ">",
                        ast::CmpOp::GtE => ">=",
                        ast::CmpOp::Is => "is",
                        ast::CmpOp::IsNot => "is not",
                        ast::CmpOp::In => "in",
                        ast::CmpOp::NotIn => "not in",
                    });
                }
                self.visit_expr(&node.left);
                for comparator in &node.comparators {
                    self.visit_expr(comparator);
                }
            }
            Expr::Call(node) => {
                self.add_operator("()");
                self.visit_expr(&node.func);
                for arg in &node.arguments.args {
                    self.visit_expr(arg);
                }
                for keyword in &node.arguments.keywords {
                    self.visit_expr(&keyword.value);
                }
            }
            Expr::FString(node) => {
                for part in &node.value {
                    if let ast::FStringPart::Literal(s) = part {
                        self.add_operand(s);
                    }
                    // Note: FStringPart::FString is not handled intentionally
                }
            }
            Expr::StringLiteral(node) => {
                self.add_operand(&node.value.to_string());
            }
            Expr::BytesLiteral(node) => {
                self.add_operand(&format!("{:?}", node.value));
            }
            Expr::NumberLiteral(node) => {
                self.add_operand(&format!("{:?}", node.value));
            }
            Expr::BooleanLiteral(node) => {
                self.add_operand(&node.value.to_string());
            }
            Expr::NoneLiteral(_) => {
                self.add_operand("None");
            }
            Expr::EllipsisLiteral(_) => {
                self.add_operand("...");
            }

            Expr::Attribute(node) => {
                self.add_operator(".");
                self.visit_expr(&node.value);
                self.add_operand(&node.attr);
            }
            Expr::Subscript(node) => {
                self.add_operator("[]");
                self.visit_expr(&node.value);
                self.visit_expr(&node.slice);
            }
            Expr::Starred(node) => {
                self.add_operator("*");
                self.visit_expr(&node.value);
            }
            Expr::Name(node) => {
                self.add_operand(&node.id);
            }
            Expr::List(node) => {
                self.add_operator("[]");
                for elt in &node.elts {
                    self.visit_expr(elt);
                }
            }
            Expr::Tuple(node) => {
                self.add_operator("()");
                for elt in &node.elts {
                    self.visit_expr(elt);
                }
            }
            Expr::Slice(node) => {
                self.add_operator(":");
                if let Some(lower) = &node.lower {
                    self.visit_expr(lower);
                }
                if let Some(upper) = &node.upper {
                    self.visit_expr(upper);
                }
                if let Some(step) = &node.step {
                    self.visit_expr(step);
                }
            }
            Expr::TString(_) | Expr::IpyEscapeCommand(_) => {}
        }
    }
}
