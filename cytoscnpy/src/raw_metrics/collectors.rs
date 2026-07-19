use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{Expr, Stmt};
use ruff_text_size::Ranged;

struct StringCollector {
    ranges: Vec<(usize, usize)>,
}

#[derive(Default)]
struct DocstringCollector {
    ranges: Vec<(usize, usize)>,
}

impl<'a> Visitor<'a> for DocstringCollector {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(node) => collect_body_docstring(&node.body, &mut self.ranges),
            Stmt::ClassDef(node) => collect_body_docstring(&node.body, &mut self.ranges),
            _ => {}
        }
        visitor::walk_stmt(self, stmt);
    }
}

#[derive(Default)]
struct LogicalLineCounter {
    count: usize,
}

impl<'a> Visitor<'a> for LogicalLineCounter {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if matches!(stmt, Stmt::Expr(node) if matches!(node.value.as_ref(), Expr::StringLiteral(_)))
        {
            return;
        }
        self.count += 1;
        visitor::walk_stmt(self, stmt);
    }
}

impl<'a> Visitor<'a> for StringCollector {
    fn visit_expr(&mut self, expr: &'a Expr) {
        self.collect_string_range(expr);
        visitor::walk_expr(self, expr);
    }
}

impl StringCollector {
    fn collect_string_range(&mut self, expr: &Expr) {
        match expr {
            Expr::StringLiteral(string) => self.push_range(string.range()),
            Expr::BytesLiteral(bytes) => self.push_range(bytes.range()),
            Expr::FString(fstring) => self.push_range(fstring.range()),
            _ => {}
        }
    }

    fn push_range(&mut self, range: ruff_text_size::TextRange) {
        self.ranges
            .push((range.start().to_usize(), range.end().to_usize()));
    }
}

pub(super) fn collect_string_ranges(body: &[Stmt]) -> Vec<(usize, usize)> {
    let mut collector = StringCollector { ranges: Vec::new() };
    for stmt in body {
        collector.visit_stmt(stmt);
    }
    collector.ranges
}

pub(super) fn collect_docstring_ranges(body: &[Stmt]) -> Vec<(usize, usize)> {
    let mut collector = DocstringCollector::default();
    collect_body_docstring(body, &mut collector.ranges);
    for stmt in body {
        collector.visit_stmt(stmt);
    }
    collector.ranges
}

fn collect_body_docstring(body: &[Stmt], ranges: &mut Vec<(usize, usize)>) {
    let Some(Stmt::Expr(node)) = body.first() else {
        return;
    };
    let Expr::StringLiteral(string) = node.value.as_ref() else {
        return;
    };
    let range = string.range();
    ranges.push((range.start().to_usize(), range.end().to_usize()));
}

pub(super) fn count_logical_lines(body: &[Stmt]) -> usize {
    let mut counter = LogicalLineCounter::default();
    for stmt in body {
        counter.visit_stmt(stmt);
    }
    counter.count
}
