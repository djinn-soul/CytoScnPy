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

    fn infer_type(expr: &Expr) -> Option<String> {
        match expr {
            Expr::StringLiteral(_) | Expr::FString(_) => Some("str".to_owned()),
            Expr::BytesLiteral(_) => Some("bytes".to_owned()),
            Expr::NumberLiteral(n) => {
                if n.value.is_int() {
                    Some("int".to_owned())
                } else {
                    Some("float".to_owned())
                }
            }
            Expr::BooleanLiteral(_) => Some("bool".to_owned()),
            Expr::NoneLiteral(_) => Some("None".to_owned()),
            Expr::List(_) | Expr::ListComp(_) => Some("list".to_owned()),
            Expr::Tuple(_) => Some("tuple".to_owned()),
            Expr::Set(_) | Expr::SetComp(_) => Some("set".to_owned()),
            Expr::Dict(_) | Expr::DictComp(_) => Some("dict".to_owned()),
            _ => None,
        }
    }

    #[allow(clippy::too_many_lines)] // This function lists all Python built-in type methods
    fn is_valid_method(type_name: &str, method_name: &str) -> bool {
        // Common protocol methods available on most types
        let protocol_methods = [
            "__len__",
            "__iter__",
            "__contains__",
            "__str__",
            "__repr__",
            "__eq__",
            "__ne__",
            "__hash__",
            "__bool__",
        ];
        if protocol_methods.contains(&method_name) {
            return true;
        }

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
            "bytes" => matches!(
                method_name,
                "capitalize"
                    | "center"
                    | "count"
                    | "decode"
                    | "endswith"
                    | "expandtabs"
                    | "find"
                    | "fromhex"
                    | "hex"
                    | "index"
                    | "isalnum"
                    | "isalpha"
                    | "isascii"
                    | "isdigit"
                    | "islower"
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
            "tuple" => matches!(method_name, "count" | "index"),
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
            "int" => matches!(
                method_name,
                "bit_length"
                    | "bit_count"
                    | "to_bytes"
                    | "from_bytes"
                    | "as_integer_ratio"
                    | "conjugate"
                    | "real"
                    | "imag"
            ),
            "float" => matches!(
                method_name,
                "as_integer_ratio"
                    | "is_integer"
                    | "hex"
                    | "fromhex"
                    | "conjugate"
                    | "real"
                    | "imag"
            ),
            "bool" | "None" => false, // These don't have meaningful mutable methods
            _ => true,                // Unknown type, assume valid to reduce false positives
        }
    }
}

impl Rule for MethodMisuseRule {
    fn name(&self) -> &'static str {
        "MethodMisuseRule"
    }

    fn code(&self) -> &'static str {
        "CSP-D601"
    }

    fn enter_stmt(&mut self, stmt: &Stmt, _context: &Context) -> Option<Vec<Finding>> {
        match stmt {
            Stmt::FunctionDef(node) => {
                self.scope_stack.push(Scope::new()); // Push scope for function
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
            Stmt::ClassDef(_) => {
                // Push scope for class - matches the pop in leave_stmt
                self.scope_stack.push(Scope::new());
            }
            Stmt::AnnAssign(node) => {
                if let Some(value) = &node.value {
                    if let Some(inferred_type) = Self::infer_type(value) {
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
                if let Some(inferred_type) = Self::infer_type(&node.value) {
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
        // Handle scope push for lambdas and comprehensions
        match expr {
            Expr::Lambda(node) => {
                self.scope_stack.push(Scope::new());
                if let Some(parameters) = &node.parameters {
                    for param in &parameters.args {
                        self.add_variable(param.parameter.name.to_string(), "unknown".to_owned());
                    }
                }
            }
            Expr::ListComp(node) => {
                self.scope_stack.push(Scope::new());
                for gen in &node.generators {
                    self.collect_targets(&gen.target);
                }
            }
            Expr::SetComp(node) => {
                self.scope_stack.push(Scope::new());
                for gen in &node.generators {
                    self.collect_targets(&gen.target);
                }
            }
            Expr::Generator(node) => {
                self.scope_stack.push(Scope::new());
                for gen in &node.generators {
                    self.collect_targets(&gen.target);
                }
            }
            Expr::DictComp(node) => {
                self.scope_stack.push(Scope::new());
                for gen in &node.generators {
                    self.collect_targets(&gen.target);
                }
            }
            _ => {}
        }

        if let Expr::Call(call) = expr {
            if let Expr::Attribute(attr) = &*call.func {
                if let Expr::Name(name_node) = &*attr.value {
                    let var_name = &name_node.id;
                    let method_name = &attr.attr;

                    if let Some(type_name) = self.get_variable_type(var_name) {
                        if !Self::is_valid_method(type_name, method_name) {
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

    fn leave_expr(&mut self, expr: &Expr, _context: &Context) -> Option<Vec<Finding>> {
        match expr {
            Expr::Lambda(_)
            | Expr::ListComp(_)
            | Expr::SetComp(_)
            | Expr::DictComp(_)
            | Expr::Generator(_) => {
                self.scope_stack.pop();
            }
            _ => {}
        }
        None
    }
}

impl MethodMisuseRule {
    fn collect_targets(&mut self, target: &Expr) {
        match target {
            Expr::Name(name) => {
                self.add_variable(name.id.to_string(), "unknown".to_owned());
            }
            Expr::Tuple(tuple) => {
                for elt in &tuple.elts {
                    self.collect_targets(elt);
                }
            }
            Expr::List(list) => {
                for elt in &list.elts {
                    self.collect_targets(elt);
                }
            }
            _ => {}
        }
    }
}
