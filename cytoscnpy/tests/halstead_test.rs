//! Tests for Halstead metrics calculation.
#![allow(clippy::unwrap_used)]

use cytoscnpy::halstead::analyze_halstead;
use ruff_python_parser::{parse, Mode};

#[test]
fn test_halstead_simple() {
    let code = "x = 1";
    let ast = parse(code, Mode::Module.into()).unwrap();
    if let ruff_python_ast::Mod::Module(m) = ast.into_syntax() {
        let metrics = analyze_halstead(&ruff_python_ast::Mod::Module(m));
        // Operators: = (1)
        // Operands: x, 1 (2)
        // n1 = 1, n2 = 2
        // N1 = 1, N2 = 2
        assert_eq!(metrics.h1, 1);
        assert_eq!(metrics.h2, 2);
        assert_eq!(metrics.n1, 1);
        assert_eq!(metrics.n2, 2);
    }
}

#[test]
fn test_halstead_function() {
    let code = "def foo(x):\n    return x + 1";
    let ast = parse(code, Mode::Module.into()).unwrap();
    if let ruff_python_ast::Mod::Module(m) = ast.into_syntax() {
        let metrics = analyze_halstead(&ruff_python_ast::Mod::Module(m));
        // Operators: def, return, + (3 distinct)
        // Operands: foo, x, 1 (3 distinct)
        // N1: def(1), return(1), +(1) = 3?
        // My visitor:
        // def foo(x): -> op: def, operand: foo, operand: x
        // return x + 1: -> op: return, op: +, operand: x, operand: 1
        // Total ops: def, return, + = 3
        // Total operands: foo, x, x, 1 = 4
        // Distinct ops: def, return, + = 3
        // Distinct operands: foo, x, 1 = 3

        assert_eq!(metrics.h1, 3);
        assert_eq!(metrics.h2, 4);
        assert_eq!(metrics.n1, 3);
        assert_eq!(metrics.n2, 3);
    }
}
