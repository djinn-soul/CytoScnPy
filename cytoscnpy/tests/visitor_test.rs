//! Tests for the AST visitor module.
#![allow(clippy::unwrap_used)]
#![allow(clippy::iter_kv_map)]

use cytoscnpy::utils::LineIndex;
use cytoscnpy::visitor::CytoScnPyVisitor;
use ruff_python_parser::{parse, Mode};
use std::collections::HashSet;
use std::path::PathBuf;

macro_rules! visit_code {
    ($code:expr, $visitor:ident) => {
        let line_index = LineIndex::new($code);
        let mut $visitor =
            CytoScnPyVisitor::new(PathBuf::from("test.py"), "test".to_string(), &line_index);
        let ast = parse($code, Mode::Module.into()).unwrap();
        if let ruff_python_ast::Mod::Module(module) = ast.into_syntax() {
            for stmt in module.body {
                $visitor.visit_stmt(&stmt);
            }
        }
    };
}

#[path = "visitor_test/cases_1.rs"]
mod cases_1;
#[path = "visitor_test/cases_2.rs"]
mod cases_2;
