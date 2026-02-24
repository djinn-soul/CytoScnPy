use ruff_python_ast::{self as ast, Stmt};

use super::HalsteadVisitor;

pub(super) fn visit_stmt(visitor: &mut HalsteadVisitor, stmt: &Stmt) {
    match stmt {
        Stmt::FunctionDef(node) => visit_function_def(visitor, node),
        Stmt::ClassDef(node) => visit_class_def(visitor, node),
        Stmt::Return(_)
        | Stmt::Delete(_)
        | Stmt::Expr(_)
        | Stmt::Pass(_)
        | Stmt::Break(_)
        | Stmt::Continue(_) => visit_simple_stmt(visitor, stmt),
        Stmt::Assign(node) => visit_assign(visitor, node),
        Stmt::AugAssign(node) => visit_aug_assign(visitor, node),
        Stmt::AnnAssign(node) => visit_ann_assign(visitor, node),
        Stmt::If(_) | Stmt::For(_) | Stmt::While(_) | Stmt::With(_) => {
            visit_control_flow(visitor, stmt);
        }
        Stmt::Raise(node) => visit_raise(visitor, node),
        Stmt::Try(node) => visit_try_stmt(visitor, node),
        Stmt::Assert(node) => visit_assert(visitor, node),
        Stmt::Import(node) => visit_import(visitor, node),
        Stmt::ImportFrom(node) => visit_import_from(visitor, node),
        Stmt::Global(node) => visit_name_list_stmt(visitor, "global", &node.names),
        Stmt::Nonlocal(node) => visit_name_list_stmt(visitor, "nonlocal", &node.names),
        _ => {}
    }
}

fn visit_function_def(visitor: &mut HalsteadVisitor, node: &ast::StmtFunctionDef) {
    if node.is_async {
        visitor.add_operator("async def");
    } else {
        visitor.add_operator("def");
    }
    visitor.add_operand(&node.name);
    for arg in &node.parameters.args {
        visitor.add_operand(&arg.parameter.name);
    }
    for stmt in &node.body {
        visitor.visit_stmt(stmt);
    }
}

fn visit_class_def(visitor: &mut HalsteadVisitor, node: &ast::StmtClassDef) {
    visitor.add_operator("class");
    visitor.add_operand(&node.name);
    for stmt in &node.body {
        visitor.visit_stmt(stmt);
    }
}

fn visit_control_flow(visitor: &mut HalsteadVisitor, stmt: &Stmt) {
    match stmt {
        Stmt::For(node) => {
            if node.is_async {
                visitor.add_operator("async for");
            } else {
                visitor.add_operator("for");
            }
            visitor.add_operator("in");
            visitor.visit_expr(&node.target);
            visitor.visit_expr(&node.iter);
            for stmt in &node.body {
                visitor.visit_stmt(stmt);
            }
        }
        Stmt::While(node) => {
            visitor.add_operator("while");
            visitor.visit_expr(&node.test);
            for stmt in &node.body {
                visitor.visit_stmt(stmt);
            }
        }
        Stmt::If(node) => {
            visitor.add_operator("if");
            visitor.visit_expr(&node.test);
            for stmt in &node.body {
                visitor.visit_stmt(stmt);
            }
            for clause in &node.elif_else_clauses {
                visitor.add_operator("else");
                for stmt in &clause.body {
                    visitor.visit_stmt(stmt);
                }
            }
        }
        Stmt::With(node) => {
            if node.is_async {
                visitor.add_operator("async with");
            } else {
                visitor.add_operator("with");
            }
            for item in &node.items {
                visitor.visit_expr(&item.context_expr);
            }
            for stmt in &node.body {
                visitor.visit_stmt(stmt);
            }
        }
        _ => {}
    }
}

fn visit_try_stmt(visitor: &mut HalsteadVisitor, node: &ast::StmtTry) {
    visitor.add_operator("try");
    for stmt in &node.body {
        visitor.visit_stmt(stmt);
    }
    for handler in &node.handlers {
        visitor.add_operator("except");
        let ast::ExceptHandler::ExceptHandler(h) = handler;
        if let Some(type_) = &h.type_ {
            visitor.visit_expr(type_);
        }
        for stmt in &h.body {
            visitor.visit_stmt(stmt);
        }
    }
    if !node.orelse.is_empty() {
        visitor.add_operator("else");
        for stmt in &node.orelse {
            visitor.visit_stmt(stmt);
        }
    }
    if !node.finalbody.is_empty() {
        visitor.add_operator("finally");
        for stmt in &node.finalbody {
            visitor.visit_stmt(stmt);
        }
    }
}

fn visit_import(visitor: &mut HalsteadVisitor, node: &ast::StmtImport) {
    visitor.add_operator("import");
    for alias in &node.names {
        visitor.add_operand(&alias.name);
        if let Some(asname) = &alias.asname {
            visitor.add_operator("as");
            visitor.add_operand(asname);
        }
    }
}

fn visit_import_from(visitor: &mut HalsteadVisitor, node: &ast::StmtImportFrom) {
    visitor.add_operator("from");
    visitor.add_operator("import");
    if let Some(module) = &node.module {
        visitor.add_operand(module);
    }
    for alias in &node.names {
        visitor.add_operand(&alias.name);
        if let Some(asname) = &alias.asname {
            visitor.add_operator("as");
            visitor.add_operand(asname);
        }
    }
}

fn visit_assign(visitor: &mut HalsteadVisitor, node: &ast::StmtAssign) {
    visitor.add_operator("=");
    for target in &node.targets {
        visitor.visit_expr(target);
    }
    visitor.visit_expr(&node.value);
}

fn visit_aug_assign(visitor: &mut HalsteadVisitor, node: &ast::StmtAugAssign) {
    visitor.add_operator(match node.op {
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
    visitor.visit_expr(&node.target);
    visitor.visit_expr(&node.value);
}

fn visit_ann_assign(visitor: &mut HalsteadVisitor, node: &ast::StmtAnnAssign) {
    visitor.add_operator(":");
    visitor.add_operator("=");
    visitor.visit_expr(&node.target);
    if let Some(value) = &node.value {
        visitor.visit_expr(value);
    }
}

fn visit_raise(visitor: &mut HalsteadVisitor, node: &ast::StmtRaise) {
    visitor.add_operator("raise");
    if let Some(exc) = &node.exc {
        visitor.visit_expr(exc);
    }
    if let Some(cause) = &node.cause {
        visitor.add_operator("from");
        visitor.visit_expr(cause);
    }
}

fn visit_assert(visitor: &mut HalsteadVisitor, node: &ast::StmtAssert) {
    visitor.add_operator("assert");
    visitor.visit_expr(&node.test);
    if let Some(msg) = &node.msg {
        visitor.visit_expr(msg);
    }
}

fn visit_simple_stmt(visitor: &mut HalsteadVisitor, stmt: &Stmt) {
    match stmt {
        Stmt::Return(node) => {
            visitor.add_operator("return");
            if let Some(value) = &node.value {
                visitor.visit_expr(value);
            }
        }
        Stmt::Delete(node) => {
            visitor.add_operator("del");
            for target in &node.targets {
                visitor.visit_expr(target);
            }
        }
        Stmt::Expr(node) => visitor.visit_expr(&node.value),
        Stmt::Pass(_) => visitor.add_operator("pass"),
        Stmt::Break(_) => visitor.add_operator("break"),
        Stmt::Continue(_) => visitor.add_operator("continue"),
        _ => {}
    }
}

fn visit_name_list_stmt(visitor: &mut HalsteadVisitor, keyword: &str, names: &[ast::Identifier]) {
    visitor.add_operator(keyword);
    for name in names {
        visitor.add_operand(name.as_str());
    }
}
