use super::{create_finding, LoopDepth, META_REGEX_LOOP};
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::{Expr, Stmt};
use ruff_text_size::Ranged;

pub(in crate::rules::quality) struct RegexLoopRule {
    loop_depth: LoopDepth,
}
impl RegexLoopRule {
    pub fn new() -> Self {
        Self {
            loop_depth: LoopDepth::new(),
        }
    }
}
impl Rule for RegexLoopRule {
    fn name(&self) -> &'static str {
        "RegexLoopRule"
    }
    fn metadata(&self) -> RuleMetadata {
        META_REGEX_LOOP
    }
    fn enter_stmt(&mut self, stmt: &Stmt, _context: &Context) -> Option<Vec<Finding>> {
        self.loop_depth.enter_stmt(stmt);
        None
    }
    fn leave_stmt(&mut self, stmt: &Stmt, _context: &Context) -> Option<Vec<Finding>> {
        self.loop_depth.leave_stmt(stmt);
        None
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if self.loop_depth.in_loop() {
            if let Expr::Call(call) = expr {
                // Check if it's a call to re.compile in a loop
                if let Expr::Attribute(attr) = &*call.func {
                    if let Expr::Name(val) = &*attr.value {
                        let mod_name = val.id.as_str();
                        let method_name = attr.attr.as_str();

                        let is_re_compile = mod_name == "re" && method_name == "compile";
                        let is_ast = mod_name == "ast" && method_name == "parse";

                        if is_re_compile || is_ast {
                            if let Some(first_arg) = call.arguments.args.first() {
                                // For re.*, first arg is pattern. For ast.parse, first arg is source.
                                // If source is a literal string and we are in a loop, it's repeated parsing of constant code.
                                // But usually ast.parse is called on variables.
                                // However, `re.compile("const")` is the target.
                                // `ast.parse("const")` is rare.
                                // But the user said "Repeated ast.parse() per file".
                                // This implies "parsing the SAME file/content multiple times".
                                // If I see `ast.parse(variable)` in a loop, it might be parsing many DIFFERENT things.
                                // But `re.compile("literal")` in a loop is definitely bad (same regex).

                                // I will strictly stick to "Literal String" for `re`.
                                // For `ast`, if the user wants "Repeated ... per file", it might mean "calling parse() multiple times on the same content".
                                // I can't check data flow.
                                // So I will only flag `ast.parse("literal")` which is silly but possible.
                                // Or `ast.parse()` inside a loop generally?
                                // "Parsing/AST hot spots... Avoid building multiple ASTs (one parse, many visitors)".
                                // This suggests `ast.parse` in a loop is suspicious if it's the *main* loop.

                                if is_re_compile && matches!(first_arg, Expr::StringLiteral(_)) {
                                    return Some(vec![create_finding(
                                        "Regex compilation/call with literal in loop (compile once outside)",
                                        META_REGEX_LOOP,
                                        context,
                                        call.range().start(),
                                        "MEDIUM",
                                    )]);
                                }

                                if is_ast {
                                    return Some(vec![create_finding(
                                        "ast.parse() called in loop (parse once and reuse AST)",
                                        META_REGEX_LOOP,
                                        context,
                                        call.range().start(),
                                        "MEDIUM",
                                    )]);
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
