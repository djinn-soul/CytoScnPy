//! Tests for Maintainability Index (MI) metrics.
#![allow(clippy::float_cmp)]

use cytoscnpy::metrics::{mi_compute, mi_rank};
use cytoscnpy::metrics::cognitive_complexity::calculate_cognitive_complexity;
use cytoscnpy::metrics::lcom4::calculate_lcom4;
use ruff_python_parser::parse_module;
use ruff_python_ast::Stmt;

#[test]
fn test_mi_compute_simple() {
    let volume = 100.0;
    let complexity = 5;
    let sloc = 20;
    let comments = 0;
    let score = mi_compute(volume, complexity, sloc, comments);
    assert!(score > 97.0 && score < 98.0);
    assert_eq!(mi_rank(score), 'A');
}

#[test]
fn test_mi_compute_with_comments() {
    let volume = 100.0;
    let complexity = 5;
    let sloc = 20;
    let comments = 5;
    let score = mi_compute(volume, complexity, sloc, comments);
    assert_eq!(score, 100.0);
    assert_eq!(mi_rank(score), 'A');
}

#[test]
fn test_mi_rank() {
    assert_eq!(mi_rank(100.0), 'A');
    assert_eq!(mi_rank(20.0), 'A');
    assert_eq!(mi_rank(19.9), 'B');
    assert_eq!(mi_rank(10.0), 'B');
    assert_eq!(mi_rank(9.9), 'C');
    assert_eq!(mi_rank(0.0), 'C');
}

fn parse_func_body(code: &str) -> Vec<Stmt> {
    let parsed = parse_module(code).unwrap();
    let module = parsed.into_syntax();
    if let Stmt::FunctionDef(f) = &module.body[0] {
        f.body.clone()
    } else {
        panic!("Expected function def");
    }
}

fn parse_class_body(code: &str) -> Vec<Stmt> {
    let parsed = parse_module(code).unwrap();
    let module = parsed.into_syntax();
    if let Stmt::ClassDef(c) = &module.body[0] {
        c.body.clone()
    } else {
        panic!("Expected class def");
    }
}

#[test]
fn test_cognitive_complexity_simple() {
    let code = "
def foo():
    if True:
        return 1
    return 0
";
    // if (+1) = 1
    let body = parse_func_body(code);
    assert_eq!(calculate_cognitive_complexity(&body), 1);
}

#[test]
fn test_cognitive_complexity_nesting() {
    let code = "
def foo():
    if True:             # +1
        if True:         # +1 (nesting=0?) No, nesting=1 -> +2 total
            print('hi')
";
    // if (+1)
    //   if (+1 + 1 nesting) = 2
    // Total = 3
    let body = parse_func_body(code);
    assert_eq!(calculate_cognitive_complexity(&body), 3);
}

#[test]
fn test_cognitive_complexity_deep() {
    let code = "
def foo():
    if condition1:                  # +1
        if condition2:              # +2 (+1 + 1 nesting)
            if condition3:          # +3 (+1 + 2 nesting)
                print('deep')
";
    // Total = 1 + 2 + 3 = 6
    let body = parse_func_body(code);
    assert_eq!(calculate_cognitive_complexity(&body), 6);
}

#[test]
fn test_cognitive_complexity_boolean_seq() {
    let code = "
def foo():
    if A and B and C:      # +1 (if) + 1 (boolean seq)
        pass
";
    // Implementation details: Visitor adds +1 for BoolOp.
    // parse_module puts A and B and C into one BoolOp usually?
    // Let's verify.
    // If it's And(A, B, C), visitor does +1.
    // Total = 1 (If) + 1 (BoolOp) = 2.
    let body = parse_func_body(code);
    assert_eq!(calculate_cognitive_complexity(&body), 2);
}

#[test]
fn test_lcom4_cohesive() {
    let code = "
class User:
    def __init__(self):
        self.name = ''
        self.email = ''
    
    def set_name(self, n):
        self.name = n
        
    def set_email(self, e):
        self.email = e
        self.validate() # calls internal
        
    def validate(self):
        print(self.name) # connects to name
";
    // Graph:
    // set_name use {name}
    // set_email use {email}, calls {validate}
    // validate use {name}
    //
    // Edges:
    // set_name -- validate (via shared 'name')
    // set_email -- validate (via Call)
    //
    // Therefore set_name -- set_email (transitive)
    // 1 component.
    let body = parse_class_body(code);
    assert_eq!(calculate_lcom4(&body), 1);
}

#[test]
fn test_lcom4_god_class() {
    let code = "
class GodClass:
    def method_a(self):
        print(self.x)
        
    def method_b(self):
        print(self.x)
        
    def method_c(self):
        print(self.y)
        
    def method_d(self):
        print(self.y)
";
    // Graph:
    // method_a -- method_b (share x)
    // method_c -- method_d (share y)
    // No connection between {a,b} and {c,d}
    // Components: 2
    let body = parse_class_body(code);
    assert_eq!(calculate_lcom4(&body), 2);
}
