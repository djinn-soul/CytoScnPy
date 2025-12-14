use crate::rules::{Context, Finding, Rule};
use ruff_python_ast::{Expr, Stmt};
use ruff_text_size::Ranged;
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct Scope {
    variables: HashMap<String, String>,
}

impl Scope {
    fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }
}

/// Rule that detects method calls on variables that don't have that method.
///
/// Uses lightweight type inference to track variable types through assignments
/// and flags method calls that are invalid for the inferred type (e.g., `str.append()`).
pub struct MethodMisuseRule {
    scope_stack: Vec<Scope>,
}

impl Default for MethodMisuseRule {
    fn default() -> Self {
        Self {
            scope_stack: vec![Scope::new()], // Global scope
        }
    }
}

impl MethodMisuseRule {
    fn get_variable_type(&self, name: &str) -> Option<&String> {
        for scope in self.scope_stack.iter().rev() {
            if let Some(type_name) = scope.variables.get(name) {
                return Some(type_name);
            }
        }
        None
    }

    fn add_variable(&mut self, name: String, type_name: String) {
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.variables.insert(name, type_name);
        }
    }

    fn infer_type(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::StringLiteral(_) => Some("str".to_owned()),
            Expr::BytesLiteral(_) => Some("bytes".to_owned()),
            Expr::NumberLiteral(n) => {
                // Check if it's an int or float
                if n.value.is_int() {
                    Some("int".to_owned())
                } else {
                    Some("float".to_owned())
                }
            }
            Expr::BooleanLiteral(_) => Some("bool".to_owned()),
            Expr::NoneLiteral(_) => Some("None".to_owned()),
            Expr::List(_) => Some("list".to_owned()),
            Expr::Tuple(_) => Some("tuple".to_owned()),
            Expr::Set(_) => Some("set".to_owned()),
            Expr::Dict(_) => Some("dict".to_owned()),
            Expr::FString(_) => Some("str".to_owned()), // f-string
            Expr::ListComp(_) => Some("list".to_owned()),
            Expr::SetComp(_) => Some("set".to_owned()),
            Expr::DictComp(_) => Some("dict".to_owned()),
            _ => None,
        }
    }

    fn is_valid_method(&self, type_name: &str, method_name: &str) -> bool {
        match type_name {
            "str" => matches!(
                method_name,
                "capitalize"
                    | "casefold"
                    | "center"
                    | "count"
                    | "encode"
                    | "endswith"
                    | "expandtabs"
                    | "find"
                    | "format"
                    | "format_map"
                    | "index"
                    | "isalnum"
                    | "isalpha"
                    | "isascii"
                    | "isdecimal"
                    | "isdigit"
                    | "isidentifier"
                    | "islower"
                    | "isnumeric"
                    | "isprintable"
                    | "isspace"
                    | "istitle"
                    | "isupper"
                    | "join"
                    | "ljust"
                    | "lower"
                    | "lstrip"
                    | "maketrans"
                    | "partition"
                    | "removeprefix"
                    | "removesuffix"
                    | "replace"
                    | "rfind"
                    | "rindex"
                    | "rjust"
                    | "rpartition"
                    | "rsplit"
                    | "rstrip"
                    | "split"
                    | "splitlines"
                    | "startswith"
                    | "strip"
                    | "swapcase"
                    | "title"
                    | "translate"
                    | "upper"
                    | "zfill"
            ),
            "list" => matches!(
                method_name,
                "append"
                    | "clear"
                    | "copy"
                    | "count"
                    | "extend"
                    | "index"
                    | "insert"
                    | "pop"
                    | "remove"
                    | "reverse"
                    | "sort"
            ),
            "dict" => matches!(
                method_name,
                "clear"
                    | "copy"
                    | "fromkeys"
                    | "get"
                    | "items"
                    | "keys"
                    | "pop"
                    | "popitem"
                    | "setdefault"
                    | "update"
                    | "values"
            ),
            "set" => matches!(
                method_name,
                "add"
                    | "clear"
                    | "copy"
                    | "difference"
                    | "difference_update"
                    | "discard"
                    | "intersection"
                    | "intersection_update"
                    | "isdisjoint"
                    | "issubset"
                    | "issuperset"
                    | "pop"
                    | "remove"
                    | "symmetric_difference"
                    | "symmetric_difference_update"
                    | "union"
                    | "update"
            ),
            "int" | "float" | "bool" | "None" => false, // Primitives mostly don't have interesting methods used like this
            // Note: int has methods like to_bytes, bit_length but rarely misused in this way to confuse with list/str
            _ => true, // Unknown type, assume valid to reduce false positives
        }
    }
}

impl Rule for MethodMisuseRule {
    fn name(&self) -> &'static str {
        "MethodMisuseRule"
    }

    fn code(&self) -> &'static str {
        "CSP-D301"
    }

    fn enter_stmt(&mut self, stmt: &Stmt, _context: &Context) -> Option<Vec<Finding>> {
        match stmt {
            Stmt::FunctionDef(node) => {
                self.scope_stack.push(Scope::new()); // Ensure we push scope!
                                                     // Track function definitions to handle return types
                                                     // We'll reset current_function when exiting (via stack or similar if full traversal)
                                                     // For now, simpler approach:
                if let Some(returns) = &node.returns {
                    if let Expr::Name(name) = &**returns {
                        // e.g. def foo() -> str:
                        // Map "foo" to "str"
                        self.add_variable(node.name.to_string(), name.id.to_string());
                    }
                }
            }
            Stmt::AnnAssign(node) => {
                if let Some(value) = &node.value {
                    if let Some(inferred_type) = self.infer_type(value) {
                        if let Expr::Name(name_node) = &*node.target {
                            if let Some(scope) = self.scope_stack.last_mut() {
                                scope
                                    .variables
                                    .insert(name_node.id.to_string(), inferred_type);
                            }
                        }
                    }
                }
            }
            // Handle regular assignments like `s = "hello"`
            Stmt::Assign(node) => {
                if let Some(value) = Some(&node.value) {
                    if let Some(inferred_type) = self.infer_type(value) {
                        for target in &node.targets {
                            if let Expr::Name(name_node) = target {
                                if let Some(scope) = self.scope_stack.last_mut() {
                                    scope
                                        .variables
                                        .insert(name_node.id.to_string(), inferred_type.clone());
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

    fn leave_stmt(&mut self, stmt: &Stmt, _context: &Context) -> Option<Vec<Finding>> {
        match stmt {
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {
                self.scope_stack.pop();
            }
            _ => {}
        }
        None
    }

    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Expr::Attribute(attr) = &*call.func {
                if let Expr::Name(name_node) = &*attr.value {
                    let var_name = &name_node.id;
                    let method_name = &attr.attr;

                    if let Some(type_name) = self.get_variable_type(var_name) {
                        if !self.is_valid_method(type_name, method_name) {
                            return Some(vec![Finding {
                                rule_id: self.code().to_owned(),
                                severity: "HIGH".to_owned(), // Method misuse is usually a runtime error
                                message: format!(
                                    "Method '{method_name}' does not exist for inferred type '{type_name}'"
                                ),
                                file: context.filename.clone(),
                                line: context.line_index.line_index(call.range().start()),
                                col: 0, // Column tracking not fully implemented in Finding yet
                            }]);
                        }
                    }
                }
            }
        }
        None
    }
}
