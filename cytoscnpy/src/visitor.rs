use crate::utils::LineIndex;
use compact_str::CompactString;
use rustc_hash::{FxHashMap, FxHashSet};
use rustpython_ast::{self as ast, Expr, Stmt};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Defines the type of scope (Module, Class, Function).
/// Uses `CompactString` for names - stores up to 24 bytes inline without heap allocation.
pub enum ScopeType {
    /// Global module scope.
    Module,
    /// Class scope with its name.
    Class(CompactString),
    /// Function scope with its name.
    Function(CompactString),
}

#[derive(Debug, Clone)]
/// Represents a symbol scope.
pub struct Scope {
    /// The type of this scope.
    pub kind: ScopeType,
    /// Set of variables defined in this scope.
    pub variables: FxHashSet<String>,
    /// Maps simple variable names to their fully qualified names in this scope.
    /// This allows us to differentiate between `x` in `func_a` and `x` in `func_b`.
    pub local_var_map: FxHashMap<String, String>,
}

impl Scope {
    /// Creates a new scope of the given type.
    pub fn new(kind: ScopeType) -> Self {
        Self {
            kind,
            variables: FxHashSet::default(),
            local_var_map: FxHashMap::default(),
        }
    }
}

/// Represents a defined entity (function, class, variable, import) in the Python code.
/// This struct holds metadata about the definition, including its location and confidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Definition {
    /// The name of the defined entity (e.g., "`my_function`").
    pub name: String,
    /// The fully qualified name (e.g., "module.class.method").
    pub full_name: String,
    /// The simple name (last part of the full name).
    pub simple_name: String,
    /// The type of definition ("function", "class", "method", "import", "variable").
    pub def_type: String,
    /// The file path where this definition resides.
    pub file: PathBuf,
    /// The line number where this definition starts.
    pub line: usize,
    /// A confidence score (0-100) indicating how certain we are that this is unused.
    /// Higher means more likely to be a valid finding.
    pub confidence: u8,
    /// The number of times this definition is referenced in the codebase.
    pub references: usize,
    /// Whether this definition is considered exported (implicitly used).
    pub is_exported: bool,
    /// Whether this definition is inside an `__init__.py` file.
    pub in_init: bool,
    /// List of base classes if this is a class definition.
    pub base_classes: Vec<String>,
    /// Whether this definition is inside an `if TYPE_CHECKING:` block.
    pub is_type_checking: bool,
    /// The cell number if this definition is from a Jupyter notebook (0-indexed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cell_number: Option<usize>,
}

impl Definition {
    /// Apply confidence penalties based on naming patterns and context.
    ///
    /// This adjusts the `confidence` score to reduce false positives.
    /// For example, private methods or dunder methods are often implicitly used,
    /// so we lower the confidence that they are "unused" even if we don't see explicit references.
    pub fn apply_penalties(&mut self) {
        let mut confidence: i16 = 100;

        // Private names (starts with _ but not __)
        // These are internal and might be used via dynamic access or just be implementation details.
        if self.simple_name.starts_with('_') && !self.simple_name.starts_with("__") {
            confidence -= 30;
        }

        // Dunder/magic methods - zero confidence
        // Python calls these implicitly (e.g., `__init__`, `__str__`).
        if self.simple_name.starts_with("__") && self.simple_name.ends_with("__") {
            confidence = 0;
        }

        // In __init__.py penalty
        // Functions and classes in `__init__.py` are often there to be exported by the package,
        // so we assume they might be used externally.
        if self.in_init && (self.def_type == "function" || self.def_type == "class") {
            confidence -= 20;
        }

        self.confidence = u8::try_from(confidence.max(0)).unwrap_or(0);
    }
}

/// The main visitor for collecting definitions and references from the AST.
pub struct CytoScnPyVisitor<'a> {
    /// Collected definitions.
    pub definitions: Vec<Definition>,
    /// Collected reference counts (name -> count). `PathBuf` removed as it was never used.
    pub references: FxHashMap<String, usize>,
    /// Names explicitly exported via `__all__`.
    pub exports: Vec<String>,
    /// Dynamic imports detected.
    pub dynamic_imports: Vec<String>,
    /// The path of the file being visited.
    pub file_path: PathBuf,
    /// The module name derived from the file path.
    pub module_name: String,
    /// Current scope stack (not fully used currently but good for tracking nested scopes).
    /// Uses SmallVec for stack allocation (most code has < 4 nested scopes).
    pub current_scope: SmallVec<[String; 4]>,
    /// Stack of class names to track current class context.
    /// Uses SmallVec - most code has < 4 nested classes.
    pub class_stack: SmallVec<[String; 4]>,
    /// Helper for line number mapping.
    pub line_index: &'a LineIndex,
    /// Map of import aliases to their original names (alias -> original).
    pub alias_map: FxHashMap<String, String>,
    /// Stack of function names to track which function we're currently inside.
    /// Uses SmallVec - most code has < 4 nested functions.
    pub function_stack: SmallVec<[String; 4]>,
    /// Map of function qualified name -> set of parameter names for that function.
    pub function_params: FxHashMap<String, FxHashSet<String>>,
    /// Stack to track if we are inside a dataclass.
    /// Uses SmallVec - most code has < 4 nested dataclasses.
    pub dataclass_stack: SmallVec<[bool; 4]>,
    /// Whether we are currently inside an `if TYPE_CHECKING:` block.
    pub in_type_checking_block: bool,
    /// Stack of scopes for variable resolution.
    /// Uses SmallVec - most code has < 8 nested scopes.
    pub scope_stack: SmallVec<[Scope; 8]>,
    /// Whether the current file is considered dynamic (e.g., uses eval/exec).
    pub is_dynamic: bool,
    /// Set of class names that have a metaclass (used to detect metaclass inheritance).
    pub metaclass_classes: FxHashSet<String>,
}

impl<'a> CytoScnPyVisitor<'a> {
    /// Creates a new visitor for the given file.
    pub fn new(file_path: PathBuf, module_name: String, line_index: &'a LineIndex) -> Self {
        Self {
            definitions: Vec::new(),
            references: FxHashMap::default(),
            exports: Vec::new(),
            dynamic_imports: Vec::new(),
            file_path,
            module_name,
            current_scope: SmallVec::new(),
            class_stack: SmallVec::new(),
            line_index,
            alias_map: FxHashMap::default(),
            function_stack: SmallVec::new(),
            function_params: FxHashMap::default(),
            dataclass_stack: SmallVec::new(),
            in_type_checking_block: false,
            scope_stack: smallvec::smallvec![Scope::new(ScopeType::Module)],
            is_dynamic: false,
            metaclass_classes: FxHashSet::default(),
        }
    }

    /// Helper to add a definition with default parameters.
    fn add_def(&mut self, name: String, def_type: &str, line: usize) {
        self.add_def_with_bases(name, def_type, line, Vec::new());
    }

    /// Pushes a new scope onto the stack.
    fn enter_scope(&mut self, scope_type: ScopeType) {
        self.scope_stack.push(Scope::new(scope_type));
    }

    /// Pops the current scope from the stack.
    fn exit_scope(&mut self) {
        self.scope_stack.pop();
    }

    /// Adds a variable definition to the current scope.
    /// Maps the simple name to its fully qualified name.
    fn add_local_def(&mut self, name: String, qualified_name: String) {
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.variables.insert(name.clone());
            scope.local_var_map.insert(name, qualified_name);
        }
    }

    /// Looks up a variable in the scope stack and returns its fully qualified name if found.
    /// Optimized to minimize allocations.
    fn resolve_name(&self, name: &str) -> Option<String> {
        for (i, scope) in self.scope_stack.iter().enumerate().rev() {
            // Class scopes are not visible to inner scopes (methods, nested classes).
            // They are only visible if they are the current (innermost) scope.
            if let ScopeType::Class(_) = &scope.kind {
                if i != self.scope_stack.len() - 1 {
                    continue;
                }
            }

            // Check local_var_map first (for function scopes with local variables)
            if let Some(qualified) = scope.local_var_map.get(name) {
                return Some(qualified.clone());
            }

            // Fallback: construct qualified name if variable exists in scope
            if scope.variables.contains(name) {
                // Pre-calculate length for capacity
                let mut total_len = name.len();
                if !self.module_name.is_empty() {
                    total_len += self.module_name.len() + 1;
                }
                for s in self.scope_stack.iter().take(i + 1).skip(1) {
                    match &s.kind {
                        ScopeType::Class(n) | ScopeType::Function(n) => {
                            total_len += n.len() + 1;
                        }
                        ScopeType::Module => {}
                    }
                }

                // Build string directly
                let mut result = String::with_capacity(total_len);
                if !self.module_name.is_empty() {
                    result.push_str(&self.module_name);
                }
                for s in self.scope_stack.iter().take(i + 1).skip(1) {
                    match &s.kind {
                        ScopeType::Class(n) | ScopeType::Function(n) => {
                            if !result.is_empty() {
                                result.push('.');
                            }
                            result.push_str(n);
                        }
                        ScopeType::Module => {}
                    }
                }
                if !result.is_empty() {
                    result.push('.');
                }
                result.push_str(name);
                return Some(result);
            }
        }
        None
    }

    /// Adds a definition to the list, applying heuristics for implicit usage.
    fn add_def_with_bases(
        &mut self,
        name: String,
        def_type: &str,
        line: usize,
        base_classes: Vec<String>,
    ) {
        let simple_name = name.split('.').next_back().unwrap_or(&name).to_owned();
        let in_init = self.file_path.ends_with("__init__.py");

        // GENERIC HEURISTICS (No hardcoded project names)

        // 1. Tests: Functions starting with 'test_' are assumed to be Pytest/Unittest tests.
        // These are run by test runners, not called explicitly.
        let is_test = simple_name.starts_with("test_");

        // 2. Dynamic Dispatch Patterns:
        //    - 'visit_' / 'leave_': Standard Visitor pattern (AST, LibCST)
        //    - 'on_': Standard Event Handler pattern (UI libs, callbacks)
        let is_dynamic_pattern = simple_name.starts_with("visit_")
            || simple_name.starts_with("leave_")
            || simple_name.starts_with("on_");

        // 3. Standard Entry Points: Common names for script execution.
        let is_standard_entry = matches!(simple_name.as_str(), "main" | "run" | "execute");

        // 4. Dunder Methods: Python's magic methods (__str__, __init__, etc.) are implicitly used.
        let is_dunder = simple_name.starts_with("__") && simple_name.ends_with("__");

        // Decision: Is this implicitly used/exported?
        let is_implicitly_used = is_test || is_dynamic_pattern || is_standard_entry || is_dunder;

        // Set reference count to 1 if implicitly used to prevent false positives.
        // This treats the definition as "used".
        let references = usize::from(is_implicitly_used);

        let definition = Definition {
            name: name.clone(),
            full_name: name,
            simple_name,
            def_type: def_type.to_owned(),
            file: self.file_path.clone(),
            line,
            confidence: 100,
            references,
            is_exported: is_implicitly_used,
            in_init,
            base_classes,
            is_type_checking: self.in_type_checking_block,
            cell_number: None,
        };

        self.definitions.push(definition);
    }

    /// Records a reference to a name by incrementing its count.
    pub fn add_ref(&mut self, name: String) {
        *self.references.entry(name).or_insert(0) += 1;
    }

    /// Constructs a qualified name based on the current scope stack.
    /// Optimized to minimize allocations by pre-calculating capacity.
    fn get_qualified_name(&self, name: &str) -> String {
        // Pre-calculate total length to avoid reallocations
        let mut total_len = name.len();

        if !self.module_name.is_empty() {
            total_len += self.module_name.len() + 1; // +1 for '.'
        }

        for scope in self.scope_stack.iter().skip(1) {
            match &scope.kind {
                ScopeType::Class(n) | ScopeType::Function(n) => {
                    total_len += n.len() + 1;
                }
                ScopeType::Module => {}
            }
        }

        // Build string with pre-allocated capacity
        let mut result = String::with_capacity(total_len);

        if !self.module_name.is_empty() {
            result.push_str(&self.module_name);
        }

        for scope in self.scope_stack.iter().skip(1) {
            match &scope.kind {
                ScopeType::Class(n) | ScopeType::Function(n) => {
                    if !result.is_empty() {
                        result.push('.');
                    }
                    result.push_str(n);
                }
                ScopeType::Module => {}
            }
        }

        if !result.is_empty() {
            result.push('.');
        }
        result.push_str(name);

        result
    }

    /// Visits function arguments (defaults and annotations).
    fn visit_arguments(&mut self, args: &ast::Arguments) {
        for arg in &args.posonlyargs {
            if let Some(default) = &arg.default {
                self.visit_expr(default);
            }
            if let Some(ann) = &arg.def.annotation {
                self.visit_expr(ann);
            }
        }
        for arg in &args.args {
            if let Some(default) = &arg.default {
                self.visit_expr(default);
            }
            if let Some(ann) = &arg.def.annotation {
                self.visit_expr(ann);
            }
        }
        if let Some(arg) = &args.vararg {
            if let Some(ann) = &arg.annotation {
                self.visit_expr(ann);
            }
        }
        for arg in &args.kwonlyargs {
            if let Some(default) = &arg.default {
                self.visit_expr(default);
            }
            if let Some(ann) = &arg.def.annotation {
                self.visit_expr(ann);
            }
        }
        if let Some(arg) = &args.kwarg {
            if let Some(ann) = &arg.annotation {
                self.visit_expr(ann);
            }
        }
    }

    /// Visits a statement node in the AST.
    pub fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            // Handle function definitions
            Stmt::FunctionDef(node) => {
                for decorator in &node.decorator_list {
                    self.visit_expr(decorator);
                }
                self.visit_arguments(&node.args);
                self.visit_function_def(&node.name, &node.args, &node.body, node.range.start());
            }
            // Handle async function definitions
            Stmt::AsyncFunctionDef(node) => {
                for decorator in &node.decorator_list {
                    self.visit_expr(decorator);
                }
                self.visit_arguments(&node.args);
                self.visit_function_def(&node.name, &node.args, &node.body, node.range.start());
            }
            // Handle class definitions
            Stmt::ClassDef(node) => {
                // Check for @dataclass decorator
                let mut is_dataclass = false;
                for decorator in &node.decorator_list {
                    self.visit_expr(decorator);
                    if let Expr::Name(name) = &decorator {
                        if name.id.as_str() == "dataclass" {
                            is_dataclass = true;
                        }
                    } else if let Expr::Call(call) = &decorator {
                        if let Expr::Name(func_name) = &*call.func {
                            if func_name.id.as_str() == "dataclass" {
                                is_dataclass = true;
                            }
                        } else if let Expr::Attribute(attr) = &*call.func {
                            if attr.attr.as_str() == "dataclass" {
                                is_dataclass = true;
                            }
                        }
                    } else if let Expr::Attribute(attr) = &decorator {
                        if attr.attr.as_str() == "dataclass" {
                            is_dataclass = true;
                        }
                    }
                }

                let name = &node.name;
                let qualified_name = self.get_qualified_name(name.as_str());
                let line = self.line_index.line_index(node.range.start());

                // Extract base class names to check for inheritance patterns later.
                let mut base_classes = Vec::new();
                for base in &node.bases {
                    match base {
                        Expr::Name(base_name) => {
                            base_classes.push(base_name.id.to_string());
                        }
                        Expr::Attribute(attr) => {
                            base_classes.push(attr.attr.to_string());
                        }
                        _ => {}
                    }
                }

                self.add_def_with_bases(
                    qualified_name.clone(),
                    "class",
                    line,
                    base_classes.clone(),
                );

                // Register class in local scope so nested classes can be resolved
                // This is critical for classes defined inside functions
                self.add_local_def(name.to_string(), qualified_name.clone());

                // Add references for base classes because inheriting uses them.
                for base in &node.bases {
                    self.visit_expr(base);
                    // Handle simple base class names mapping to module refs
                    if let Expr::Name(base_name) = base {
                        self.add_ref(base_name.id.to_string()); // Also add simple reference
                        if !self.module_name.is_empty() {
                            let qualified_base = format!("{}.{}", self.module_name, base_name.id);
                            self.add_ref(qualified_base);
                        }
                    }
                }

                // Visit keyword arguments (e.g., metaclass=SomeClass)
                // This ensures classes used as metaclasses are tracked as "used"
                let mut has_metaclass = false;
                for keyword in &node.keywords {
                    self.visit_expr(&keyword.value);
                    // Check if this is a metaclass keyword
                    if keyword.arg.as_ref().map(rustpython_ast::Identifier::as_str)
                        == Some("metaclass")
                    {
                        has_metaclass = true;
                    }
                    // Also add direct reference for simple name metaclasses
                    if let Expr::Name(kw_name) = &keyword.value {
                        self.add_ref(kw_name.id.to_string());
                        if !self.module_name.is_empty() {
                            let qualified_kw = format!("{}.{}", self.module_name, kw_name.id);
                            self.add_ref(qualified_kw);
                        }
                    }
                }

                // Track classes that have a metaclass (for inheritance detection)
                if has_metaclass {
                    self.metaclass_classes.insert(name.to_string());
                    // Also add qualified name
                    self.metaclass_classes.insert(qualified_name.clone());
                }

                // Check if this class inherits from a metaclass class (registry pattern)
                // If so, mark this class as implicitly used (side-effect registration)
                for base_class in &base_classes {
                    if self.metaclass_classes.contains(base_class) {
                        // This class is registered via metaclass side-effect, mark as used
                        self.add_ref(qualified_name);
                        self.add_ref(name.to_string());
                        break;
                    }
                }

                // Push class name to stack for nested definitions (methods/inner classes).
                self.class_stack.push(name.to_string());
                self.dataclass_stack.push(is_dataclass);

                // Enter class scope
                self.enter_scope(ScopeType::Class(CompactString::from(name.as_str())));

                // Visit class body.
                for stmt in &node.body {
                    self.visit_stmt(stmt);
                }
                // Pop class name after visiting body.
                self.class_stack.pop();
                self.dataclass_stack.pop();

                // Exit class scope
                self.exit_scope();
            }
            // Handle imports
            Stmt::Import(node) => {
                for alias in &node.names {
                    let asname = alias.asname.as_ref().unwrap_or(&alias.name);
                    let line = self.line_index.line_index(node.range.start());
                    self.add_def(asname.to_string(), "import", line);
                    self.add_local_def(asname.to_string(), asname.to_string());

                    // Add alias mapping: asname -> name
                    self.alias_map
                        .insert(asname.to_string(), alias.name.to_string());
                }
            }
            // Handle 'from ... import'
            Stmt::ImportFrom(node) => {
                // Ignore __future__ imports to prevent false "unused import" positives.
                // `from __future__ import ...` is a compiler directive, not a real import.
                if let Some(module) = &node.module {
                    if module == "__future__" {
                        return;
                    }
                }

                let line = self.line_index.line_index(node.range.start());
                for alias in &node.names {
                    let asname = alias.asname.as_ref().unwrap_or(&alias.name);
                    self.add_def(asname.to_string(), "import", line);
                    self.add_local_def(asname.to_string(), asname.to_string());

                    // Add alias mapping: asname -> module.name (if module exists) or just name
                    if let Some(module) = &node.module {
                        let full_name = format!("{}.{}", module, alias.name);
                        self.add_ref(full_name.clone());
                        self.alias_map.insert(asname.to_string(), full_name);
                    } else {
                        self.alias_map
                            .insert(asname.to_string(), alias.name.to_string());
                    }
                }
            }
            // Handle assignments
            Stmt::Assign(node) => {
                // Handle __all__ exports. `__all__ = ["a", "b"]` explicitly exports names.
                if let Some(Expr::Name(target)) = node.targets.first() {
                    if target.id.as_str() == "__all__" {
                        if let Expr::List(list) = &*node.value {
                            for elt in &list.elts {
                                if let Expr::Constant(constant) = elt {
                                    if let ast::Constant::Str(s) = &constant.value {
                                        self.exports.push(s.clone());
                                    }
                                }
                            }
                        }
                    }
                }
                // First visit RHS for references
                self.visit_expr(&node.value);

                // Track variable definitions
                for target in &node.targets {
                    if let Expr::Name(name_node) = target {
                        // Skip __all__ as it was already handled above
                        if name_node.id.as_str() != "__all__" {
                            let qualified_name = self.get_qualified_name(&name_node.id);
                            let line = self.line_index.line_index(node.range.start());
                            self.add_def(qualified_name.clone(), "variable", line);
                            self.add_local_def(name_node.id.to_string(), qualified_name);
                        }
                    } else {
                        // For non-name targets, visit for references
                        self.visit_expr(target);
                    }
                }
            }
            // Handle augmented assignments (+=, -=, etc.)
            Stmt::AugAssign(node) => {
                self.visit_expr(&node.target);
                self.visit_expr(&node.value);
            }
            // Handle annotated assignments (x: int = 1)
            Stmt::AnnAssign(node) => {
                // Track variable definition
                if let Expr::Name(name_node) = &*node.target {
                    let qualified_name = self.get_qualified_name(&name_node.id);
                    let line = self.line_index.line_index(node.range.start());
                    self.add_def(qualified_name.clone(), "variable", line);
                    self.add_local_def(name_node.id.to_string(), qualified_name.clone());

                    // If inside a dataclass, mark as implicitly used (field)
                    if let Some(true) = self.dataclass_stack.last() {
                        // Only if we are in a class (which we should be if dataclass_stack is true)
                        // and NOT in a function (class fields are at class level)
                        if !self.class_stack.is_empty() && self.function_stack.is_empty() {
                            self.add_ref(qualified_name);
                        }
                    }
                } else {
                    // For non-name targets, visit for references
                    self.visit_expr(&node.target);
                }

                self.visit_expr(&node.annotation);
                if let Some(value) = &node.value {
                    self.visit_expr(value);
                }
            }
            // Handle expression statements
            Stmt::Expr(node) => {
                self.visit_expr(&node.value);
            }
            // Control Flow Handling - traverse bodies recursively
            Stmt::If(node) => {
                // Check for TYPE_CHECKING guard
                let mut is_type_checking_guard = false;
                if let Expr::Name(name) = &*node.test {
                    if name.id.as_str() == "TYPE_CHECKING" {
                        is_type_checking_guard = true;
                    } else if let Some(original) = self.alias_map.get(name.id.as_str()) {
                        if original.ends_with("TYPE_CHECKING") {
                            is_type_checking_guard = true;
                        }
                    }
                } else if let Expr::Attribute(attr) = &*node.test {
                    if attr.attr.as_str() == "TYPE_CHECKING" {
                        if let Expr::Name(base) = &*attr.value {
                            if base.id.as_str() == "typing"
                                || base.id.as_str() == "typing_extensions"
                            {
                                is_type_checking_guard = true;
                            } else if let Some(original) = self.alias_map.get(base.id.as_str()) {
                                if original == "typing" || original == "typing_extensions" {
                                    is_type_checking_guard = true;
                                }
                            }
                        }
                    }
                }

                self.visit_expr(&node.test);

                let prev_in_type_checking = self.in_type_checking_block;
                if is_type_checking_guard {
                    self.in_type_checking_block = true;
                }

                for stmt in &node.body {
                    self.visit_stmt(stmt);
                }

                // Restore previous state
                self.in_type_checking_block = prev_in_type_checking;

                for stmt in &node.orelse {
                    self.visit_stmt(stmt);
                }
            }
            Stmt::For(node) => {
                self.visit_expr(&node.iter);
                for stmt in &node.body {
                    self.visit_stmt(stmt);
                }
                for stmt in &node.orelse {
                    self.visit_stmt(stmt);
                }
            }
            Stmt::AsyncFor(node) => {
                self.visit_expr(&node.iter);
                for stmt in &node.body {
                    self.visit_stmt(stmt);
                }
                for stmt in &node.orelse {
                    self.visit_stmt(stmt);
                }
            }
            Stmt::While(node) => {
                self.visit_expr(&node.test);
                for stmt in &node.body {
                    self.visit_stmt(stmt);
                }
                for stmt in &node.orelse {
                    self.visit_stmt(stmt);
                }
            }
            Stmt::With(node) => {
                for item in &node.items {
                    self.visit_expr(&item.context_expr);
                }
                for stmt in &node.body {
                    self.visit_stmt(stmt);
                }
            }
            Stmt::AsyncWith(node) => {
                for item in &node.items {
                    self.visit_expr(&item.context_expr);
                }
                for stmt in &node.body {
                    self.visit_stmt(stmt);
                }
            }
            Stmt::Try(node) => {
                for stmt in &node.body {
                    self.visit_stmt(stmt);
                }
                for ast::ExceptHandler::ExceptHandler(handler_node) in &node.handlers {
                    if let Some(exc) = &handler_node.type_ {
                        self.visit_expr(exc);
                    }
                    for stmt in &handler_node.body {
                        self.visit_stmt(stmt);
                    }
                }
                for stmt in &node.orelse {
                    self.visit_stmt(stmt);
                }
                for stmt in &node.finalbody {
                    self.visit_stmt(stmt);
                }
            }
            Stmt::TryStar(node) => {
                for stmt in &node.body {
                    self.visit_stmt(stmt);
                }
                for ast::ExceptHandler::ExceptHandler(handler_node) in &node.handlers {
                    if let Some(exc) = &handler_node.type_ {
                        self.visit_expr(exc);
                    }
                    for stmt in &handler_node.body {
                        self.visit_stmt(stmt);
                    }
                }
                for stmt in &node.orelse {
                    self.visit_stmt(stmt);
                }
                for stmt in &node.finalbody {
                    self.visit_stmt(stmt);
                }
            }
            Stmt::Return(node) => {
                if let Some(value) = &node.value {
                    self.visit_expr(value);
                }
            }
            Stmt::Assert(node) => {
                self.visit_expr(&node.test);
                if let Some(msg) = &node.msg {
                    self.visit_expr(msg);
                }
            }
            Stmt::Raise(node) => {
                if let Some(exc) = &node.exc {
                    self.visit_expr(exc);
                }
                if let Some(cause) = &node.cause {
                    self.visit_expr(cause);
                }
            }
            Stmt::Delete(node) => {
                for target in &node.targets {
                    self.visit_expr(target);
                }
            }
            Stmt::Match(node) => {
                self.visit_expr(&node.subject);
                for case in &node.cases {
                    self.visit_match_pattern(&case.pattern);
                    if let Some(guard) = &case.guard {
                        self.visit_expr(guard);
                    }
                    for stmt in &case.body {
                        self.visit_stmt(stmt);
                    }
                }
            }
            _ => {}
        }
    }

    // Helper function to handle shared logic between FunctionDef and AsyncFunctionDef
    fn visit_function_def(
        &mut self,
        name: &str,
        args: &ast::Arguments,
        body: &[Stmt],
        range_start: rustpython_ast::TextSize,
    ) {
        let qualified_name = self.get_qualified_name(name);
        let line = self.line_index.line_index(range_start);

        // Determine if it's a function or a method based on class stack.
        let def_type = if self.class_stack.is_empty() {
            "function"
        } else {
            "method"
        };

        self.add_def(qualified_name.clone(), def_type, line);

        // Enter function scope
        self.enter_scope(ScopeType::Function(CompactString::from(name)));

        // Track parameters
        let mut param_names = FxHashSet::default();

        // Helper to extract parameter name
        let extract_param_name = |arg: &ast::ArgWithDefault| -> String { arg.def.arg.to_string() };

        // Positional-only parameters
        for arg in &args.posonlyargs {
            let param_name = extract_param_name(arg);
            param_names.insert(param_name.clone());
            let param_qualified = if param_name != "self" && param_name != "cls" {
                format!("{qualified_name}.{param_name}")
            } else {
                param_name.clone()
            };
            self.add_local_def(param_name.clone(), param_qualified.clone());

            // Skip self and cls - they're implicit
            if param_name != "self" && param_name != "cls" {
                self.add_def(param_qualified, "parameter", line);
            }
        }

        // Regular positional parameters
        for arg in &args.args {
            let param_name = extract_param_name(arg);
            param_names.insert(param_name.clone());
            let param_qualified = if param_name != "self" && param_name != "cls" {
                format!("{qualified_name}.{param_name}")
            } else {
                param_name.clone()
            };
            self.add_local_def(param_name.clone(), param_qualified.clone());

            // Skip self and cls
            if param_name != "self" && param_name != "cls" {
                self.add_def(param_qualified, "parameter", line);
            }
        }

        // Keyword-only parameters
        for arg in &args.kwonlyargs {
            let param_name = extract_param_name(arg);
            param_names.insert(param_name.clone());
            let param_qualified = format!("{qualified_name}.{param_name}");
            self.add_local_def(param_name.clone(), param_qualified.clone());
            self.add_def(param_qualified, "parameter", line);
        }

        // *args parameter
        if let Some(vararg) = &args.vararg {
            let param_name = vararg.arg.to_string();
            param_names.insert(param_name.clone());
            let param_qualified = format!("{qualified_name}.{param_name}");
            self.add_local_def(param_name, param_qualified.clone());
            self.add_def(param_qualified, "parameter", line);
        }

        // **kwargs parameter
        if let Some(kwarg) = &args.kwarg {
            let param_name = kwarg.arg.to_string();
            param_names.insert(param_name.clone());
            let param_qualified = format!("{qualified_name}.{param_name}");
            self.add_local_def(param_name, param_qualified.clone());
            self.add_def(param_qualified, "parameter", line);
        }

        // Store parameters for this function
        self.function_params
            .insert(qualified_name.clone(), param_names);

        // Push function onto stack (for nested functions)
        self.function_stack.push(qualified_name);

        // Visit function body
        for stmt in body {
            self.visit_stmt(stmt);
        }

        // Pop function stack
        self.function_stack.pop();

        // Exit function scope
        self.exit_scope();
    }

    /// Visits an expression node in the AST.
    pub fn visit_expr(&mut self, expr: &Expr) {
        match expr {
            // Name usage (variable access)
            Expr::Name(node) => {
                if node.ctx.is_load() {
                    let name = node.id.to_string();

                    // Try to resolve using scope stack first
                    if let Some(qualified) = self.resolve_name(&name) {
                        self.add_ref(qualified);
                    } else {
                        // If not found in local scope, assume it's a global or builtin.
                        // We qualify it with the module name to avoid matching class attributes
                        // via the simple name fallback in the analyzer.
                        if self.module_name.is_empty() {
                            self.add_ref(name.clone());
                        } else {
                            self.add_ref(format!("{}.{}", self.module_name, name));
                        }
                    }

                    // Check aliases - resolve aliased imports to their original names.
                    // Clone to release borrow of alias_map before calling add_ref (borrow checker fix).
                    if let Some(original) = self.alias_map.get(&name).cloned() {
                        // Add simple name first if original is qualified (e.g., "os.path" -> "path")
                        if let Some(simple) = original.split('.').next_back() {
                            if simple != original {
                                self.add_ref(simple.to_owned());
                            }
                        }
                        // Now move the owned string (no clone needed)
                        self.add_ref(original);
                    }
                }
            }
            // Function call
            Expr::Call(node) => {
                // Check for dynamic execution or reflection
                if let Expr::Name(func_name) = &*node.func {
                    let name = func_name.id.as_str();
                    if name == "eval" || name == "exec" || name == "globals" || name == "locals" {
                        self.is_dynamic = true;
                    }

                    // Special handling for hasattr(obj, "attr") to detect attribute usage
                    if name == "hasattr" && node.args.len() == 2 {
                        // Extract the object (first arg) and attribute name (second arg)
                        if let (Expr::Name(obj_name), Expr::Constant(attr_const)) =
                            (&node.args[0], &node.args[1])
                        {
                            if let ast::Constant::Str(attr_str) = &attr_const.value {
                                // Construct the qualified attribute name
                                // e.g., hasattr(Colors, "GREEN") -> Colors.GREEN
                                let attr_ref = format!("{}.{}", obj_name.id, attr_str);
                                self.add_ref(attr_ref);

                                // Also try with module prefix
                                if !self.module_name.is_empty() {
                                    let full_attr_ref = format!(
                                        "{}.{}.{}",
                                        self.module_name, obj_name.id, attr_str
                                    );
                                    self.add_ref(full_attr_ref);
                                }
                            }
                        }
                    }
                }

                self.visit_expr(&node.func);
                for arg in &node.args {
                    self.visit_expr(arg);
                }
                // Don't forget keyword arguments (e.g., func(a=b))
                for keyword in &node.keywords {
                    self.visit_expr(&keyword.value);
                }
            }
            // Attribute access (e.g., obj.attr)
            Expr::Attribute(node) => {
                // Always track the attribute name as a reference (loose tracking)
                // This ensures we catch methods in chains like `obj.method().other_method()`
                self.add_ref(node.attr.to_string());

                if let Expr::Name(name_node) = &*node.value {
                    let base_id = name_node.id.as_str();

                    // Check if base_id is an alias
                    // Fix: Done Clone the string to avoid holding borrow of self.alias_map while calling self.add_ref
                    let original_base_opt = self.alias_map.get(base_id).cloned();
                    if let Some(original_base) = original_base_opt {
                        // e.g. l -> lib
                        // Add ref to lib
                        self.add_ref(original_base.clone());

                        // Add ref to lib.attr
                        let full_attr = format!("{}.{}", original_base, node.attr);
                        self.add_ref(full_attr);
                    }

                    // Case 1: Strict self.method usage inside a class context.
                    // We want to track references to methods of the current class.
                    if (base_id == "self" || base_id == "cls") && !self.class_stack.is_empty() {
                        let method_name = &node.attr;
                        let mut parts = Vec::new();
                        if !self.module_name.is_empty() {
                            parts.push(self.module_name.clone());
                        }
                        parts.extend(self.class_stack.clone());
                        parts.push(method_name.to_string());
                        let qualified = parts.join(".");
                        self.add_ref(qualified);
                    }
                    // Case 2: External usage (obj.method or sys.exit)
                    else {
                        // Track "sys" from "sys.exit" (Fixes unused import)
                        self.add_ref(base_id.to_owned());

                        // Track "sys.exit" (Specific attribute access)
                        let full_attr = format!("{}.{}", base_id, node.attr);
                        self.add_ref(full_attr);
                    }
                }
                self.visit_expr(&node.value);
            }
            // FIX: Done Dynamic Dispatch / String References
            Expr::Constant(node) => {
                if let ast::Constant::Str(s) = &node.value {
                    // Heuristic: If a string looks like a simple identifier or dotted path (no spaces),
                    // track it as a reference. This helps with getattr(self, "visit_" + name)
                    // and stringified type hints like "models.User".
                    if !s.contains(' ') && !s.is_empty() {
                        self.add_ref(s.clone());
                    }
                }
            }
            // Recursion Boilerplate - Ensure we visit children of all other expressions
            Expr::BoolOp(node) => {
                for value in &node.values {
                    self.visit_expr(value);
                }
            }
            Expr::BinOp(node) => {
                self.visit_expr(&node.left);
                self.visit_expr(&node.right);
            }
            Expr::UnaryOp(node) => {
                self.visit_expr(&node.operand);
            }
            Expr::Lambda(node) => {
                self.visit_expr(&node.body);
            }
            Expr::IfExp(node) => {
                self.visit_expr(&node.test);
                self.visit_expr(&node.body);
                self.visit_expr(&node.orelse);
            }
            Expr::Dict(node) => {
                for (key, value) in node.keys.iter().zip(&node.values) {
                    if let Some(k) = key {
                        self.visit_expr(k);
                    }
                    self.visit_expr(value);
                }
            }
            Expr::Set(node) => {
                for elt in &node.elts {
                    self.visit_expr(elt);
                }
            }
            Expr::ListComp(node) => {
                self.visit_expr(&node.elt);
                for gen in &node.generators {
                    self.visit_expr(&gen.iter);
                    for if_expr in &gen.ifs {
                        self.visit_expr(if_expr);
                    }
                }
            }
            Expr::SetComp(node) => {
                self.visit_expr(&node.elt);
                for gen in &node.generators {
                    self.visit_expr(&gen.iter);
                    for if_expr in &gen.ifs {
                        self.visit_expr(if_expr);
                    }
                }
            }
            Expr::DictComp(node) => {
                self.visit_expr(&node.key);
                self.visit_expr(&node.value);
                for gen in &node.generators {
                    self.visit_expr(&gen.iter);
                    for if_expr in &gen.ifs {
                        self.visit_expr(if_expr);
                    }
                }
            }
            Expr::GeneratorExp(node) => {
                self.visit_expr(&node.elt);
                for gen in &node.generators {
                    self.visit_expr(&gen.iter);
                    for if_expr in &gen.ifs {
                        self.visit_expr(if_expr);
                    }
                }
            }
            Expr::Await(node) => self.visit_expr(&node.value),
            Expr::Yield(node) => {
                if let Some(value) = &node.value {
                    self.visit_expr(value);
                }
            }
            Expr::YieldFrom(node) => self.visit_expr(&node.value),
            Expr::Compare(node) => {
                self.visit_expr(&node.left);
                for comparator in &node.comparators {
                    self.visit_expr(comparator);
                }
            }
            Expr::Subscript(node) => {
                self.visit_expr(&node.value);
                self.visit_expr(&node.slice);
            }
            Expr::FormattedValue(node) => self.visit_expr(&node.value),
            Expr::JoinedStr(node) => {
                for value in &node.values {
                    self.visit_expr(value);
                }
            }
            Expr::List(node) => {
                for elt in &node.elts {
                    self.visit_expr(elt);
                }
            }
            Expr::Tuple(node) => {
                for elt in &node.elts {
                    self.visit_expr(elt);
                }
            }
            Expr::Slice(node) => {
                if let Some(lower) = &node.lower {
                    self.visit_expr(lower);
                }
                if let Some(upper) = &node.upper {
                    self.visit_expr(upper);
                }
                if let Some(step) = &node.step {
                    self.visit_expr(step);
                }
            }
            // Handle starred expressions (*args in function calls)
            // This ensures when *args is passed to a function, args is marked as used
            Expr::Starred(node) => {
                self.visit_expr(&node.value);
            }
            _ => {}
        }
    }

    /// Helper to recursively visit match patterns
    fn visit_match_pattern(&mut self, pattern: &ast::Pattern) {
        match pattern {
            ast::Pattern::MatchValue(node) => {
                self.visit_expr(&node.value);
            }
            ast::Pattern::MatchSingleton(_) => {
                // Literals (None, True, False) - nothing to track
            }
            ast::Pattern::MatchSequence(node) => {
                for p in &node.patterns {
                    self.visit_match_pattern(p);
                }
            }
            ast::Pattern::MatchMapping(node) => {
                for (key, value) in node.keys.iter().zip(&node.patterns) {
                    self.visit_expr(key);
                    self.visit_match_pattern(value);
                }
                if let Some(rest) = &node.rest {
                    let qualified_name = self.get_qualified_name(rest);
                    let line = self.line_index.line_index(node.range.start());
                    self.add_def(qualified_name.clone(), "variable", line);
                    // Add to local scope so it can be resolved when used
                    self.add_local_def(rest.to_string(), qualified_name);
                }
            }
            ast::Pattern::MatchClass(node) => {
                self.visit_expr(&node.cls);
                for p in &node.patterns {
                    self.visit_match_pattern(p);
                }
                for p in &node.kwd_patterns {
                    self.visit_match_pattern(p);
                }
            }
            ast::Pattern::MatchStar(node) => {
                if let Some(name) = &node.name {
                    let qualified_name = self.get_qualified_name(name);
                    let line = self.line_index.line_index(node.range.start());
                    self.add_def(qualified_name.clone(), "variable", line);
                    // Add to local scope so it can be resolved when used
                    self.add_local_def(name.to_string(), qualified_name);
                }
            }
            ast::Pattern::MatchAs(node) => {
                if let Some(pattern) = &node.pattern {
                    self.visit_match_pattern(pattern);
                }
                if let Some(name) = &node.name {
                    let qualified_name = self.get_qualified_name(name);
                    let line = self.line_index.line_index(node.range.start());
                    self.add_def(qualified_name.clone(), "variable", line);
                    // Add to local scope so it can be resolved when used
                    self.add_local_def(name.to_string(), qualified_name);
                }
            }
            ast::Pattern::MatchOr(node) => {
                for p in &node.patterns {
                    self.visit_match_pattern(p);
                }
            }
        }
    }
}
