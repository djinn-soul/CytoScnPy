mod function_visitor;
mod metrics;
mod visitor;

use ruff_python_ast::{self as ast};

use function_visitor::FunctionHalsteadVisitor;
use visitor::HalsteadVisitor;

pub use metrics::HalsteadMetrics;

/// Calculates Halstead metrics for a given AST module.
pub fn analyze_halstead(ast: &ast::Mod) -> HalsteadMetrics {
    let mut visitor = HalsteadVisitor::new();
    visitor.visit_mod(ast);
    visitor.calculate_metrics()
}

/// Calculates Halstead metrics directly from a parsed `ModModule`.
///
/// Avoids the full-AST `Mod::Module(module.clone())` deep clone that callers
/// would otherwise need to satisfy `analyze_halstead`'s `&Mod` signature.
pub fn analyze_halstead_module(module: &ast::ModModule) -> HalsteadMetrics {
    let mut visitor = HalsteadVisitor::new();
    for stmt in &module.body {
        visitor.visit_stmt(stmt);
    }
    visitor.calculate_metrics()
}

/// Calculates Halstead metrics for each function in a given AST module.
pub fn analyze_halstead_functions(ast: &ast::Mod) -> Vec<(String, HalsteadMetrics)> {
    let mut visitor = FunctionHalsteadVisitor::new();
    visitor.visit_mod(ast);
    visitor.results
}
