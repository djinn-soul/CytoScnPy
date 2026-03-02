use super::super::utils::{create_finding, get_call_name};
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::{Expr, Stmt};
use ruff_text_size::Ranged;

/// Rule for detecting hardcoded network binds to all interfaces.
pub struct HardcodedBindAllInterfacesRule {
    /// Rule metadata.
    pub metadata: RuleMetadata,
}
impl HardcodedBindAllInterfacesRule {
    /// Creates a new bind-all rule instance.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for HardcodedBindAllInterfacesRule {
    fn name(&self) -> &'static str {
        "HardcodedBindAllInterfacesRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    fn enter_stmt(&mut self, stmt: &Stmt, context: &Context) -> Option<Vec<Finding>> {
        match stmt {
            Stmt::Assign(assign) => {
                let is_host_bind = assign.targets.iter().any(|target| {
                    if let Expr::Name(name) = target {
                        let name = name.id.to_lowercase();
                        name.contains("host") || name.contains("bind") || name == "listen_addr"
                    } else {
                        false
                    }
                });
                if is_host_bind {
                    if let Expr::StringLiteral(string) = &*assign.value {
                        let value = string.value.to_string();
                        if value == "0.0.0.0" || value == "::" {
                            return Some(vec![create_finding(
                                "Possible hardcoded binding to all interfaces (0.0.0.0 or ::)",
                                self.metadata,
                                context,
                                assign.value.range().start(),
                                "MEDIUM",
                            )]);
                        }
                    }
                }
            }
            Stmt::AnnAssign(ann_assign) => {
                if let Expr::Name(name) = &*ann_assign.target {
                    let name = name.id.to_lowercase();
                    if name.contains("host") || name.contains("bind") || name == "listen_addr" {
                        if let Some(value) = &ann_assign.value {
                            if let Expr::StringLiteral(string) = &**value {
                                let value_text = string.value.to_string();
                                if value_text == "0.0.0.0" || value_text == "::" {
                                    return Some(vec![create_finding("Possible hardcoded binding to all interfaces (0.0.0.0 or ::)", self.metadata, context, value.range().start(), "MEDIUM")]);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        None
    }

    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            for keyword in &call.arguments.keywords {
                if let Some(arg_name) = &keyword.arg {
                    if arg_name == "host" || arg_name == "bind" {
                        if let Expr::StringLiteral(string) = &keyword.value {
                            let value = string.value.to_string();
                            if value == "0.0.0.0" || value == "::" {
                                return Some(vec![create_finding(
                                    "Possible hardcoded binding to all interfaces (0.0.0.0 or ::)",
                                    self.metadata,
                                    context,
                                    keyword.value.range().start(),
                                    "MEDIUM",
                                )]);
                            }
                        }
                    }
                }
            }
            if let Some(name) = get_call_name(&call.func) {
                if (name == "bind" || name.ends_with(".bind")) && !call.arguments.args.is_empty() {
                    if let Expr::Tuple(tuple) = &call.arguments.args[0] {
                        if !tuple.elts.is_empty() {
                            if let Expr::StringLiteral(string) = &tuple.elts[0] {
                                let value = string.value.to_string();
                                if value == "0.0.0.0" || value == "::" {
                                    return Some(vec![create_finding("Possible hardcoded binding to all interfaces (0.0.0.0 or ::)", self.metadata, context, tuple.elts[0].range().start(), "MEDIUM")]);
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }
}
