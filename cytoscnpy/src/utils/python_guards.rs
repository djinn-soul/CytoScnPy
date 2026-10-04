//! Exact recognition of Python executable entry guards.
use ruff_python_ast::{CmpOp, Expr};

pub(crate) fn is_main_guard(expr: &Expr) -> bool {
    let Expr::Compare(compare) = expr else {
        return false;
    };
    if compare.ops.len() != 1 || compare.ops[0] != CmpOp::Eq || compare.comparators.len() != 1 {
        return false;
    }
    let right = &compare.comparators[0];
    (is_name(&compare.left) && is_main(right)) || (is_main(&compare.left) && is_name(right))
}

fn is_name(expr: &Expr) -> bool {
    matches!(expr, Expr::Name(name) if name.id.as_str() == "__name__")
}

fn is_main(expr: &Expr) -> bool {
    matches!(expr, Expr::StringLiteral(value) if value.value.to_str() == "__main__")
}
