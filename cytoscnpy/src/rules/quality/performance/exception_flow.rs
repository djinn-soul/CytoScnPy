use super::{create_finding, LoopDepth, META_EXCEPTION_FLOW_LOOP};
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

pub(in crate::rules::quality) struct ExceptionFlowInLoopRule {
    loop_depth: LoopDepth,
}
impl ExceptionFlowInLoopRule {
    pub fn new() -> Self {
        Self {
            loop_depth: LoopDepth::new(),
        }
    }
}
impl Rule for ExceptionFlowInLoopRule {
    fn name(&self) -> &'static str {
        "ExceptionFlowInLoopRule"
    }
    fn metadata(&self) -> RuleMetadata {
        META_EXCEPTION_FLOW_LOOP
    }
    fn enter_stmt(&mut self, stmt: &Stmt, context: &Context) -> Option<Vec<Finding>> {
        self.loop_depth.enter_stmt(stmt);

        if self.loop_depth.in_loop() {
            if let Stmt::Try(try_stmt) = stmt {
                for handler in &try_stmt.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = handler;
                    if let Some(type_expr) = &h.type_ {
                        let mut is_target_exception = false;
                        match &**type_expr {
                            Expr::Name(name) => {
                                if matches!(
                                    name.id.as_str(),
                                    "KeyError" | "AttributeError" | "IndexError"
                                ) {
                                    is_target_exception = true;
                                }
                            }
                            Expr::Tuple(tuple) => {
                                for elt in &tuple.elts {
                                    if let Expr::Name(name) = elt {
                                        if matches!(
                                            name.id.as_str(),
                                            "KeyError" | "AttributeError" | "IndexError"
                                        ) {
                                            is_target_exception = true;
                                            break;
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }

                        if is_target_exception && try_stmt.body.len() <= 2 {
                            return Some(vec![create_finding(
                                "Using try-except for control flow in loop (KeyError/AttributeError is slower than .get() or hasattr())",
                                META_EXCEPTION_FLOW_LOOP,
                                context,
                                stmt.range().start(),
                                "LOW",
                            )]);
                        }
                    }
                }
            }
        }
        None
    }
    fn leave_stmt(&mut self, stmt: &Stmt, _context: &Context) -> Option<Vec<Finding>> {
        self.loop_depth.leave_stmt(stmt);
        None
    }
}
