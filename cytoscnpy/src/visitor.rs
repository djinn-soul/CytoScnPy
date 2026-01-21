use crate::constants::MAX_RECURSION_DEPTH;
use crate::constants::PYTEST_HOOKS;
use crate::utils::LineIndex;
use compact_str::CompactString;
use regex::Regex;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use smallvec::SmallVec;
use std::path::PathBuf;
use std::sync::Arc;

/// Serialize Arc<PathBuf> as a plain `PathBuf` for JSON output
fn serialize_arc_path<S>(path: &Arc<PathBuf>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    path.as_ref().serialize(serializer)
}

/// Deserialize a plain `PathBuf` into Arc<PathBuf>
fn deserialize_arc_path<'de, D>(deserializer: D) -> Result<Arc<PathBuf>, D::Error>
where
    D: Deserializer<'de>,
{
    PathBuf::deserialize(deserializer).map(Arc::new)
}

/// Serialize `SmallVec`<[String; 2]> as a plain Vec<String> for JSON output
fn serialize_smallvec_string<S>(
    vec: &SmallVec<[String; 2]>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    vec.as_slice().serialize(serializer)
}

/// Deserialize a plain Vec<String> into `SmallVec`<[String; 2]>
fn deserialize_smallvec_string<'de, D>(deserializer: D) -> Result<SmallVec<[String; 2]>, D::Error>
where
    D: Deserializer<'de>,
{
    Vec::<String>::deserialize(deserializer).map(SmallVec::from_vec)
}
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
    /// Whether this scope is managed by a framework (e.g., a decorated function).
    pub is_framework: bool,
    /// Variables explicitly declared as global in this scope.
    pub global_declarations: FxHashSet<String>,
}

impl Scope {
    /// Creates a new scope of the given type.
    #[must_use]
    pub fn new(kind: ScopeType) -> Self {
        Self {
            kind,
            variables: FxHashSet::default(),
            local_var_map: FxHashMap::default(),
            is_framework: false,
            global_declarations: FxHashSet::default(),
        }
    }
}

/// Represents a defined entity (function, class, variable, import) in the Python code.
/// This struct holds metadata about the definition, including its location and confidence.
/// Argument struct for adding a definition to reduce argument count.
#[derive(Debug, Clone)]
pub struct DefinitionInfo {
    /// The name of the defined entity.
    pub name: String,
    /// The type of definition ("function", "class", "variable", etc.).
    pub def_type: String,
    /// The starting line number (1-indexed).
    pub line: usize,
    /// The ending line number (1-indexed).
    pub end_line: usize,
    /// The starting column number (1-indexed).
    pub col: usize,
    /// The starting byte offset.
    pub start_byte: usize,
    /// The ending byte offset.
    pub end_byte: usize,
    /// The starting byte offset of the full definition (including decorators/keywords) for fix generation.
    pub full_start_byte: usize,
    /// Base classes (for class definitions), empty for others.
    pub base_classes: SmallVec<[String; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[allow(clippy::struct_excessive_bools)]
/// A fully resolved definition found during analysis.
///
/// This struct contains all metadata about a definition, including its
/// location, type, usage references, and any associated issues or fixes.
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
    /// Uses `Arc` to avoid cloning for every definition in the same file.
    #[serde(
        serialize_with = "serialize_arc_path",
        deserialize_with = "deserialize_arc_path"
    )]
    pub file: Arc<PathBuf>,
    /// The line number where this definition starts.
    pub line: usize,
    /// The line number where this definition ends.
    pub end_line: usize,
    /// The starting column number (1-indexed).
    pub col: usize,
    /// The starting byte offset (0-indexed).
    pub start_byte: usize,
    /// The ending byte offset (exclusive).
    pub end_byte: usize,

    /// A confidence score (0-100) indicating how certain we are that this is unused.
    /// Higher means more likely to be a valid finding.
    pub confidence: u8,
    /// The number of times this definition is referenced in the codebase.
    pub references: usize,
    /// Whether this definition is considered exported (implicitly used).
    pub is_exported: bool,
    /// Whether this definition is inside an `__init__.py` file.
    pub in_init: bool,
    /// Whether this definition is managed by a framework (e.g. inside a decorated function).
    pub is_framework_managed: bool,
    /// List of base classes if this is a class definition.
    /// Uses `SmallVec<[String; 2]>` - most classes have 0-2 base classes.
    #[serde(
        serialize_with = "serialize_smallvec_string",
        deserialize_with = "deserialize_smallvec_string"
    )]
    pub base_classes: SmallVec<[String; 2]>,
    /// Whether this definition is inside an `if TYPE_CHECKING:` block.
    pub is_type_checking: bool,
    /// Whether this definition is captured by a nested scope (closure).
    pub is_captured: bool,
    /// The cell number if this definition is from a Jupyter notebook (0-indexed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cell_number: Option<usize>,
    /// Whether this method only references itself (recursive with no external callers).
    /// Used for class-method linking to identify truly unused recursive methods.
    #[serde(default)]
    pub is_self_referential: bool,
    /// Human-readable message for this finding (generated based on `def_type`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Optional fix suggestion with byte ranges for surgical code removal.
    /// Only populated when running with CST analysis enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<Box<crate::analyzer::types::FixSuggestion>>,
    /// Whether this definition is a member of an Enum class.
    /// Used to allow simple name matching for Enum members (e.g. `Status.ACTIVE` matching `ACTIVE`).
    #[serde(default)]
    pub is_enum_member: bool,
    /// Whether this definition is a module-level constant (`UPPER_CASE`).
    #[serde(default)]
    pub is_constant: bool,
    /// Whether this definition is a potential secret/key.
    #[serde(default)]
    pub is_potential_secret: bool,
}

// apply_penalties method removed as it was redundant with heuristics.rs

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
    /// Uses `Arc` to share with all definitions without cloning.
    pub file_path: Arc<PathBuf>,
    /// The module name derived from the file path.
    pub module_name: String,
    /// Current scope stack (not fully used currently but good for tracking nested scopes).
    /// Uses `SmallVec` for stack allocation (most code has < 4 nested scopes).
    pub current_scope: SmallVec<[String; 4]>,
    /// Stack of class names to track current class context.
    /// Uses `SmallVec` - most code has < 4 nested classes.
    pub class_stack: SmallVec<[String; 4]>,
    /// Helper for line number mapping.
    pub line_index: &'a LineIndex,
    /// Map of import aliases to their original names (alias -> original).
    pub alias_map: FxHashMap<String, String>,
    /// Stack of function names to track which function we're currently inside.
    /// Uses `SmallVec` - most code has < 4 nested functions.
    pub function_stack: SmallVec<[String; 4]>,
    /// Map of function qualified name -> set of parameter names for that function.
    pub function_params: FxHashMap<String, FxHashSet<String>>,
    /// Stack to track if we are inside a model class (dataclass, Pydantic, etc.).
    /// Uses `SmallVec` - most code has < 4 nested classes.
    pub model_class_stack: SmallVec<[bool; 4]>,
    /// Whether we are currently inside an `if TYPE_CHECKING:` block.
    pub in_type_checking_block: bool,
    /// Stack of scopes for variable resolution.
    /// Uses `SmallVec` - most code has < 8 nested scopes.
    pub scope_stack: SmallVec<[Scope; 8]>,
    /// Set of scopes that contain dynamic execution (eval/exec).
    /// Stores the fully qualified name of the scope.
    pub dynamic_scopes: FxHashSet<String>,
    /// Variables that are captured by nested scopes (closures).
    pub captured_definitions: FxHashSet<String>,
    /// Set of class names that have a metaclass (used to detect metaclass inheritance).
    pub metaclass_classes: FxHashSet<String>,
    /// Set of method qualified names that are self-referential (recursive).
    /// Used for class-method linking to detect methods that only call themselves.
    pub self_referential_methods: FxHashSet<String>,
    /// Cached scope prefix for faster qualified name building.
    /// Updated on scope push/pop to avoid rebuilding on every `resolve_name` call.
    cached_scope_prefix: String,
    /// Current recursion depth for ``visit_stmt`/`visit_expr`` to prevent stack overflow.
    depth: usize,
    /// Whether the recursion limit was hit during traversal.
    pub recursion_limit_hit: bool,
    /// Set of names that are automatically called by frameworks (e.g., `main`, `setup`, `teardown`).
    auto_called: FxHashSet<&'static str>,
    /// Stack to track if we are inside a Protocol class (PEP 544).
    /// Uses `SmallVec` - most code has < 4 nested classes.
    pub protocol_class_stack: SmallVec<[bool; 4]>,
    /// Stack to track if we are inside an Enum class.
    /// Uses `SmallVec` - most code has < 4 nested classes.
    pub enum_class_stack: SmallVec<[bool; 4]>,
    /// Whether we are currently inside a try...except ``ImportError`` block.
    pub in_import_error_block: bool,
    /// Stack to track if we are inside an ABC-inheriting class.
    pub abc_class_stack: SmallVec<[bool; 4]>,
    /// Map of ABC class name -> set of abstract method names defined in that class.
    pub abc_abstract_methods: FxHashMap<String, FxHashSet<String>>,
    /// Map of Protocol class name -> set of method names defined in that class.
    pub protocol_methods: FxHashMap<String, FxHashSet<String>>,
    /// Detected optional dependency flags (HAS_*, HAVE_*) inside except ``ImportError`` blocks.
    pub optional_dependency_flags: FxHashSet<String>,
}

impl<'a> CytoScnPyVisitor<'a> {
    /// Creates a new visitor for the given file.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn new(file_path: PathBuf, module_name: String, line_index: &'a LineIndex) -> Self {
        let file_path = Arc::new(file_path); // Wrap in Arc once, share everywhere
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
            model_class_stack: SmallVec::new(),
            in_type_checking_block: false,
            scope_stack: smallvec::smallvec![Scope::new(ScopeType::Module)],
            dynamic_scopes: FxHashSet::default(),
            captured_definitions: FxHashSet::default(),
            metaclass_classes: FxHashSet::default(),
            self_referential_methods: FxHashSet::default(),
            cached_scope_prefix: String::new(),
            depth: 0,
            recursion_limit_hit: false,
            auto_called: PYTEST_HOOKS().clone(),
            protocol_class_stack: SmallVec::new(),
            enum_class_stack: SmallVec::new(),
            in_import_error_block: false,
            abc_class_stack: SmallVec::new(),
            abc_abstract_methods: FxHashMap::default(),
            protocol_methods: FxHashMap::default(),
            optional_dependency_flags: FxHashSet::default(),
        }
    }

    /// Helper to extract range info (`start_line`, `end_line`, `col`, `start_byte`, `end_byte`) from a node.
    fn get_range_info<T: Ranged>(&self, node: &T) -> (usize, usize, usize, usize, usize) {
        let range = node.range();
        let start_byte = range.start().to_usize();
        let end_byte = range.end().to_usize();
        // line_index uses 1-based indexing for lines
        let start_line = self.line_index.line_index(range.start());
        let end_line = self.line_index.line_index(range.end());
        let col = self.line_index.column_index(range.start());
        (start_line, end_line, col, start_byte, end_byte)
    }

    /// Helper to add a definition using the info struct.
    #[allow(clippy::too_many_lines)]
    fn add_definition(&mut self, info: DefinitionInfo) {
        let simple_name = info
            .name
            .split('.')
            .next_back()
            .unwrap_or(&info.name)
            .to_owned();
        let in_init = self.file_path.ends_with("__init__.py");

        // GENERIC HEURISTICS (No hardcoded project names)

        // 1. Tests: Functions starting with 'test_' are assumed to be Pytest/Unittest tests.
        // These are run by test runners, not called explicitly.
        // 1. Tests: Vulture-style Smart Heuristic
        // If the file looks like a test (tests/ or test_*.py), we are lenient.
        let file_is_test = crate::utils::is_test_path(&self.file_path.to_string_lossy());
        let is_test_function = simple_name.starts_with("test_");

        let is_test_class =
            file_is_test && (simple_name.contains("Test") || simple_name.contains("Suite"));

        let is_test = is_test_function || is_test_class;

        // 2. Dynamic Dispatch Patterns:
        //    - 'visit_' / 'leave_': Standard Visitor pattern (AST, LibCST)
        //    - 'on_': Standard Event Handler pattern (UI libs, callbacks)
        let is_dynamic_pattern = simple_name.starts_with("visit_")
            || simple_name.starts_with("leave_")
            || simple_name.starts_with("on_");

        // 3. Standard Entry Points: Common names for script execution.
        let is_standard_entry = matches!(simple_name.as_str(), "main" | "run" | "execute");

        // Check for module-level constants (UPPER_CASE)
        // These are often configuration or exported constants.
        // BUT exclude potential secrets/keys which should be detected if unused.
        let is_potential_secret = simple_name.contains("KEY")
            || simple_name.contains("SECRET")
            || simple_name.contains("PASS")
            || simple_name.contains("TOKEN");

        // 5. Public API: Symbols not starting with '_' are considered exported/public API.
        //    This is crucial for library analysis where entry points aren't explicit.
        //    FIX: Secrets are NOT public API - they should be flagged if unused.
        let is_public_api =
            !simple_name.starts_with('_') && info.def_type != "method" && !is_potential_secret;

        // 4. Dunder Methods: Python's magic methods (__str__, __init__, etc.) are implicitly used.
        let is_dunder = simple_name.starts_with("__") && simple_name.ends_with("__");

        // Check if this is a public class attribute (e.g., `class MyClass: my_attr = 1`)
        let is_class_scope = self
            .scope_stack
            .last()
            .is_some_and(|s| matches!(s.kind, ScopeType::Class(_)));

        // Strict Enum Check: Enum members are NOT implicitly used. They must be referenced.
        let is_enum_member = self.enum_class_stack.last().copied().unwrap_or(false);

        let is_public_class_attr = is_class_scope
            && info.def_type == "variable"
            && !simple_name.starts_with('_')
            && !is_enum_member;

        let is_constant = self.scope_stack.len() == 1
            && info.def_type == "variable"
            && !simple_name.starts_with('_')
            && !is_potential_secret
            && simple_name.chars().all(|c| !c.is_lowercase())
            && simple_name.chars().any(char::is_uppercase);

        // Decision: Is this implicitly used? (For reference counting/suppression)
        let is_implicitly_used = is_test
            || is_dynamic_pattern
            || is_standard_entry
            || is_dunder
            || is_public_class_attr
            || self.auto_called.contains(simple_name.as_str());

        // FIX: Global constants (UPPER_CASE) are NOT "implicitly used" (which would hide them forever).
        // Instead, we let them fall through as unused, BUT we will assign them very low confidence later.
        // This allows --confidence 0 to find unused settings, while keeping default runs clean.

        // Decision: Is this exported? (For Semantic Graph roots)
        let is_exported = is_implicitly_used || is_public_api;

        // Set reference count to 1 if implicitly used to prevent false positives.
        // This treats the definition as "used".
        let references = usize::from(is_implicitly_used);

        // FIX: Ensure the references map is updated for implicitly used items
        // This prevents single_file.rs from overwriting the references count with 0
        if is_implicitly_used {
            self.add_ref(info.name.clone());
        }

        // Generate human-readable message based on def_type
        let message = match info.def_type.as_str() {
            "method" => format!("Method '{simple_name}' is defined but never used"),
            "class" => format!("Class '{simple_name}' is defined but never used"),
            "import" => format!("'{simple_name}' is imported but never used"),
            "variable" => format!("Variable '{simple_name}' is assigned but never used"),
            "parameter" => format!("Parameter '{simple_name}' is never used"),
            _ => format!("'{simple_name}' is defined but never used"),
        };

        // Try to create a fix suggestion if we have valid CST ranges
        // This ensures the JS extension gets ranges even if CST module didn't run
        let fix = if info.full_start_byte < info.end_byte {
            Some(Box::new(crate::analyzer::types::FixSuggestion::deletion(
                info.full_start_byte,
                info.end_byte,
            )))
        } else {
            None
        };

        let is_enum_member = self.enum_class_stack.last().copied().unwrap_or(false);

        let definition = Definition {
            name: info.name.clone(),
            full_name: info.name,
            simple_name,
            def_type: info.def_type,
            file: Arc::clone(&self.file_path), // O(1) Arc clone instead of O(n) PathBuf clone
            line: info.line,
            end_line: info.end_line,
            col: info.col,
            start_byte: info.start_byte,
            end_byte: info.end_byte,
            confidence: 100,
            references,
            is_exported,
            in_init,
            is_framework_managed: self.scope_stack.last().is_some_and(|s| s.is_framework),
            base_classes: info.base_classes,
            is_type_checking: self.in_type_checking_block,
            is_captured: false,
            cell_number: None,
            is_enum_member,

            is_self_referential: false,
            message: Some(message),
            fix,
            is_constant,
            is_potential_secret,
        };

        self.definitions.push(definition);
    }

    /// Pushes a new scope onto the stack and updates cached prefix.
    fn enter_scope(&mut self, scope_type: ScopeType) {
        // Update cached prefix based on scope type
        match &scope_type {
            ScopeType::Class(name) | ScopeType::Function(name) => {
                if !self.cached_scope_prefix.is_empty() {
                    self.cached_scope_prefix.push('.');
                }
                self.cached_scope_prefix.push_str(name);
            }
            ScopeType::Module => {}
        }
        self.scope_stack.push(Scope::new(scope_type));
    }

    /// Pops the current scope from the stack and updates cached prefix.
    fn exit_scope(&mut self) {
        if let Some(scope) = self.scope_stack.pop() {
            // Remove this scope's contribution from cached prefix
            match &scope.kind {
                ScopeType::Class(name) | ScopeType::Function(name) => {
                    // Remove ".name" or just "name" if at start
                    let name_len = name.len();
                    if self.cached_scope_prefix.len() > name_len {
                        // Has a dot before it
                        self.cached_scope_prefix
                            .truncate(self.cached_scope_prefix.len() - name_len - 1);
                    } else {
                        // It's the only thing in the prefix
                        self.cached_scope_prefix
                            .truncate(self.cached_scope_prefix.len() - name_len);
                    }
                }
                ScopeType::Module => {}
            }
        }
    }

    /// Adds a variable definition to the current scope.
    /// Maps the simple name to its fully qualified name.
    fn add_local_def(&mut self, name: String, qualified_name: String) {
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.variables.insert(name.clone());
            scope.local_var_map.insert(name, qualified_name);
        }
    }

    /// Looks up a variable in the scope stack and returns its fully qualified name and scope index if found.
    /// Optimized: uses `cached_scope_prefix` for innermost scope to avoid rebuilding.
    fn resolve_name_with_info(&self, name: &str) -> Option<(String, usize)> {
        let innermost_idx = self.scope_stack.len() - 1;

        for (i, scope) in self.scope_stack.iter().enumerate().rev() {
            // Class scopes are not visible to inner scopes (methods, nested classes).
            // They are only visible if they are the current (innermost) scope.
            if let ScopeType::Class(_) = &scope.kind {
                if i != innermost_idx {
                    continue;
                }
            }

            // Check local_var_map first (for function scopes with local variables)
            if let Some(qualified) = scope.local_var_map.get(name) {
                return Some((qualified.clone(), i));
            }

            // Fallback: construct qualified name if variable exists in scope
            if scope.variables.contains(name) {
                // Fast path: if this is the innermost scope, use cached prefix
                if i == innermost_idx {
                    if self.cached_scope_prefix.is_empty() {
                        return Some((name.to_owned(), i));
                    }
                    let mut result =
                        String::with_capacity(self.cached_scope_prefix.len() + 1 + name.len());
                    result.push_str(&self.cached_scope_prefix);
                    result.push('.');
                    result.push_str(name);
                    return Some((result, i));
                }

                // Slow path: build prefix up to scope at index i
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
                return Some((result, i));
            }
        }
        None
    }

    /// Optimized: uses `cached_scope_prefix` for innermost scope to avoid rebuilding.
    fn resolve_name(&self, name: &str) -> Option<String> {
        self.resolve_name_with_info(name).map(|(q, _)| q)
    }

    /// Records a reference to a name by incrementing its count.
    pub fn add_ref(&mut self, name: String) {
        *self.references.entry(name).or_insert(0) += 1;
    }

    /// Returns the fully qualified ID of the current scope.
    /// Used for tracking dynamic scopes.
    fn get_current_scope_id(&self) -> String {
        if self.cached_scope_prefix.is_empty() {
            self.module_name.clone()
        } else if self.module_name.is_empty() {
            self.cached_scope_prefix.clone()
        } else {
            format!("{}.{}", self.module_name, self.cached_scope_prefix)
        }
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
    fn visit_arguments(&mut self, args: &ast::Parameters) {
        // Visit positional-only args
        for arg in &args.posonlyargs {
            if let Some(ann) = &arg.parameter.annotation {
                self.visit_expr(ann);
            }
            if let Some(default) = &arg.default {
                self.visit_expr(default);
            }
        }
        // Visit regular args
        for arg in &args.args {
            if let Some(ann) = &arg.parameter.annotation {
                self.visit_expr(ann);
            }
            if let Some(default) = &arg.default {
                self.visit_expr(default);
            }
        }
        // Visit *args
        if let Some(arg) = &args.vararg {
            if let Some(ann) = &arg.annotation {
                self.visit_expr(ann);
            }
        }
        // Visit keyword-only args
        for arg in &args.kwonlyargs {
            if let Some(ann) = &arg.parameter.annotation {
                self.visit_expr(ann);
            }
            if let Some(default) = &arg.default {
                self.visit_expr(default);
            }
        }
        // Visit **kwargs
        if let Some(arg) = &args.kwarg {
            if let Some(ann) = &arg.annotation {
                self.visit_expr(ann);
            }
        }
    }

    /// Visits a statement node in the AST.
    ///
    /// This is the core statement visitor that handles all Python statement types.
    /// It is intentionally monolithic due to Rust's borrow checker constraints -
    /// extracting helper methods would require complex lifetime annotations or
    /// splitting the visitor state.
    ///
    /// # Structure
    ///
    /// The function handles these statement types in order:
    /// - **`FunctionDef`** (line ~606): Delegates to `visit_function_def` helper
    /// - **`ClassDef`** (line ~629): Class definitions with decorators, bases, metaclasses
    /// - **Import/ImportFrom** (line ~764): Module imports and alias tracking
    /// - **Assign/AugAssign/AnnAssign** (line ~823): Variable definitions
    /// - **Control flow** (line ~908): If/For/While/With/Try/Match statements
    /// - **Return/Assert/Raise/Delete** (line ~1008): Other statements
    ///
    /// # Recursion Safety
    ///
    /// Uses `MAX_RECURSION_DEPTH` guard to prevent stack overflow on deeply nested code.
    #[allow(clippy::too_many_lines)]
    pub fn visit_stmt(&mut self, stmt: &Stmt) {
        // Recursion depth guard to prevent stack overflow on deeply nested code
        if self.depth >= MAX_RECURSION_DEPTH {
            self.recursion_limit_hit = true;
            return;
        }
        self.depth += 1;

        match stmt {
            // Handle function definitions (both sync and async - ruff uses is_async flag)
            Stmt::FunctionDef(node) => {
                // Visit decorators (ruff uses Decorator type with .expression field)
                for decorator in &node.decorator_list {
                    self.visit_expr(&decorator.expression);
                }
                // Use .parameters instead of .args in ruff
                self.visit_arguments(&node.parameters);
                // Visit return annotation to track string type hints like -> "OrderedDict"
                if let Some(returns) = &node.returns {
                    self.visit_expr(returns);
                }
                self.visit_function_def(
                    &node.name,
                    &node.decorator_list,
                    &node.parameters,
                    &node.body,
                    node.range(),
                );
            }
            // Handle class definitions
            Stmt::ClassDef(node) => {
                // Check if this class is a "Model Class" (Pydantic, Dataclass, TypedDict, etc.)
                let mut is_model_class = false;
                for decorator in &node.decorator_list {
                    self.visit_expr(&decorator.expression);
                    // Check the inner expression for dataclass decorator
                    if let Expr::Name(name) = &decorator.expression {
                        if name.id.as_str() == "dataclass" {
                            is_model_class = true;
                        }
                    } else if let Expr::Call(call) = &decorator.expression {
                        if let Expr::Name(func_name) = &*call.func {
                            if func_name.id.as_str() == "dataclass" {
                                is_model_class = true;
                            }
                        } else if let Expr::Attribute(attr) = &*call.func {
                            if attr.attr.as_str() == "dataclass" {
                                is_model_class = true;
                            }
                        }
                    } else if let Expr::Attribute(attr) = &decorator.expression {
                        if attr.attr.as_str() == "dataclass" {
                            is_model_class = true;
                        } else if attr.attr.as_str() == "s" {
                            // attr.s
                            is_model_class = true;
                        }
                    }
                }

                let name = &node.name;
                let qualified_name = self.get_qualified_name(name.as_str());
                // Use the name's range for the finding line (skips decorators)
                let name_line = self.line_index.line_index(name.range().start());
                let name_col = self.line_index.column_index(name.range().start());
                // Use full node range for end_line and fix ranges (includes decorators)
                let (_, end_line, _, start_byte, end_byte) = self.get_range_info(node);

                // Extract base class names to check for inheritance patterns later.
                // In ruff, node.bases() returns an iterator over base class expressions
                let bases = node.bases();
                let mut base_classes: SmallVec<[String; 2]> = SmallVec::new();
                for base in bases {
                    match base {
                        Expr::Name(base_name) => {
                            let b_name = base_name.id.as_str();
                            base_classes.push(b_name.to_owned());
                            // Check base classes for Model behaviors
                            if matches!(
                                b_name,
                                "BaseModel" | "TypedDict" | "NamedTuple" | "Protocol" | "Struct"
                            ) {
                                is_model_class = true;
                            }
                        }
                        Expr::Attribute(attr) => {
                            let b_name = attr.attr.as_str();
                            base_classes.push(b_name.to_owned());
                            // Check base classes for Model behaviors
                            if matches!(
                                b_name,
                                "BaseModel" | "TypedDict" | "NamedTuple" | "Protocol" | "Struct"
                            ) {
                                is_model_class = true;
                            }
                        }
                        _ => {}
                    }
                }

                // Check if this is a Protocol class
                let is_protocol = base_classes
                    .iter()
                    .any(|b| b == "Protocol" || b.ends_with(".Protocol"));
                self.protocol_class_stack.push(is_protocol);

                // Check if this is an ABC class
                // Note: .ABC is a Python class name pattern, not a file extension (false positive from clippy)
                #[allow(clippy::case_sensitive_file_extension_comparisons)]
                let is_abc = base_classes
                    .iter()
                    .any(|b| b == "ABC" || b == "abc.ABC" || b.ends_with(".ABC"));
                self.abc_class_stack.push(is_abc);

                // Check if this is an Enum class
                let is_enum = base_classes.iter().any(|b| {
                    matches!(
                        b.as_str(),
                        "Enum"
                            | "IntEnum"
                            | "StrEnum"
                            | "enum.Enum"
                            | "enum.IntEnum"
                            | "enum.StrEnum"
                    )
                });
                self.enum_class_stack.push(is_enum);

                self.add_definition(DefinitionInfo {
                    name: qualified_name.clone(),
                    def_type: "class".to_owned(),
                    line: name_line,
                    end_line,
                    col: name_col,
                    start_byte: node.name.range.start().to_usize(),
                    end_byte,
                    full_start_byte: start_byte,
                    base_classes: base_classes.clone(),
                });

                // Register class in local scope so nested classes can be resolved
                // This is critical for classes defined inside functions
                self.add_local_def(name.to_string(), qualified_name.clone());

                // Add references for base classes because inheriting uses them.
                for base in node.bases() {
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
                for keyword in node.keywords() {
                    self.visit_expr(&keyword.value);
                    // Check if this is a metaclass keyword (use as_str() directly)
                    if keyword
                        .arg
                        .as_ref()
                        .map(ruff_python_ast::Identifier::as_str)
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
                self.model_class_stack.push(is_model_class);

                // Enter class scope
                self.enter_scope(ScopeType::Class(CompactString::from(name.as_str())));

                // Visit class body.
                for stmt in &node.body {
                    self.visit_stmt(stmt);
                }
                // Pop class name after visiting body.
                self.class_stack.pop();
                self.model_class_stack.pop();
                self.protocol_class_stack.pop();
                self.abc_class_stack.pop();
                self.enum_class_stack.pop();

                // Exit class scope
                self.exit_scope();
            }
            // Handle imports
            Stmt::Import(node) => {
                // Add each import to definitions
                for alias in &node.names {
                    // imports often don't have AS names, so we check
                    let simple_name = alias.asname.as_ref().unwrap_or(&alias.name);
                    let (line, end_line, col, start_byte, end_byte) = self.get_range_info(alias);

                    self.add_definition(DefinitionInfo {
                        name: simple_name.to_string(),
                        def_type: "import".to_owned(),
                        line,
                        end_line,
                        col,
                        start_byte,
                        end_byte,
                        full_start_byte: start_byte,
                        base_classes: SmallVec::new(),
                    });

                    self.add_local_def(
                        simple_name.as_str().to_owned(),
                        simple_name.as_str().to_owned(),
                    );

                    // Add alias mapping: asname -> name
                    self.alias_map
                        .insert(simple_name.to_string(), alias.name.to_string());

                    // Optional dependency tracking
                    if self.in_import_error_block {
                        let qualified_name = if self.module_name.is_empty() {
                            simple_name.to_string()
                        } else {
                            format!("{}.{}", self.module_name, simple_name)
                        };
                        self.add_ref(qualified_name);
                        // Also add simple alias for matching import definitions
                        self.add_ref(simple_name.to_string());
                    }
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

                for alias in &node.names {
                    let asname = alias.asname.as_ref().unwrap_or(&alias.name);
                    let (line, end_line, col, start_byte, end_byte) = self.get_range_info(alias);

                    self.add_definition(DefinitionInfo {
                        name: asname.to_string(),
                        def_type: "import".to_owned(),
                        line,
                        end_line,
                        col,
                        start_byte,
                        end_byte,
                        full_start_byte: start_byte,
                        base_classes: SmallVec::new(),
                    });
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

                    // Optional dependency tracking
                    if self.in_import_error_block {
                        let qualified_name = if self.module_name.is_empty() {
                            asname.to_string()
                        } else {
                            format!("{}.{}", self.module_name, asname)
                        };
                        self.add_ref(qualified_name);
                        // Also add simple alias for matching import definitions
                        self.add_ref(asname.to_string());
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
                                if let Expr::StringLiteral(string_lit) = elt {
                                    self.exports.push(string_lit.value.to_string());
                                }
                            }
                        }
                    }
                }

                // Track HAS_*/HAVE_* flags in except blocks
                if self.in_import_error_block {
                    for target in &node.targets {
                        if let Expr::Name(name_node) = target {
                            let id = name_node.id.as_str();
                            if id.starts_with("HAS_") || id.starts_with("HAVE_") {
                                self.optional_dependency_flags.insert(id.to_owned());
                                // Mark as used
                                self.add_ref(id.to_owned());
                                if !self.module_name.is_empty() {
                                    let qualified = format!("{}.{}", self.module_name, id);
                                    self.add_ref(qualified);
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
                            // Check if this variable is declared global in the current scope
                            let is_global = self.scope_stack.last().is_some_and(|s| {
                                s.global_declarations.contains(name_node.id.as_str())
                            });

                            if is_global {
                                // It's a write to a global variable. Mark the global as referenced.
                                self.add_ref(name_node.id.to_string());
                                if !self.module_name.is_empty() {
                                    let qualified =
                                        format!("{}.{}", self.module_name, name_node.id);
                                    self.add_ref(qualified);
                                }
                            } else {
                                let qualified_name = self.get_qualified_name(&name_node.id);
                                let (line, end_line, col, start_byte, end_byte) =
                                    self.get_range_info(name_node);
                                // Clone for add_def, move to add_local_def (last use)
                                self.add_definition(DefinitionInfo {
                                    name: qualified_name.clone(),
                                    def_type: "variable".to_owned(),
                                    line,
                                    end_line,
                                    col,
                                    start_byte,
                                    end_byte,
                                    full_start_byte: start_byte,
                                    base_classes: SmallVec::new(),
                                });
                                self.add_local_def(
                                    name_node.id.to_string(),
                                    qualified_name.clone(),
                                );

                                // Fix for unannotated class attributes in Model Classes (Dataclasses, Pydantic)
                                // If we are in a Model Class and not in a method, treat this assignment as a field definition
                                // and mark it as implicitly used.
                                if !self.class_stack.is_empty() && self.function_stack.is_empty() {
                                    if let Some(true) = self.model_class_stack.last() {
                                        self.add_ref(qualified_name);
                                    }
                                }
                            }
                        }
                    } else {
                        // For non-name targets, visit for references (e.g. self.x = 1)
                        self.visit_expr(target);
                    }
                }

                // Check for TypeAliasType (PEP 695 backport) or NewType
                // Shape = TypeAliasType("Shape", "tuple[int, int]")
                // UserId = NewType("UserId", int)
                if let Expr::Call(call) = &*node.value {
                    if let Expr::Name(func_name) = &*call.func {
                        let fname = func_name.id.as_str();
                        if fname == "TypeAliasType" || fname == "NewType" {
                            // Mark targets as used (they are type definitions)
                            for target in &node.targets {
                                if let Expr::Name(name_node) = target {
                                    let qualified_name = self.get_qualified_name(&name_node.id);
                                    self.add_ref(qualified_name);
                                }
                            }
                        }
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
                    let (line, end_line, col, start_byte, end_byte) =
                        self.get_range_info(name_node);
                    self.add_definition(DefinitionInfo {
                        name: qualified_name.clone(),
                        def_type: "variable".to_owned(),
                        line,
                        end_line,
                        col,
                        start_byte,
                        end_byte,
                        full_start_byte: start_byte,
                        base_classes: SmallVec::new(),
                    });
                    self.add_local_def(name_node.id.to_string(), qualified_name.clone());

                    // If we are in a Model Class (Dataclasses, Pydantic) and not in a method,
                    // treat this assignment as a field definition and mark it as implicitly used.
                    if !self.class_stack.is_empty() && self.function_stack.is_empty() {
                        if let Some(true) = self.model_class_stack.last() {
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

                // Check for TypeAlias annotation
                // Shape: TypeAlias = tuple[int, int]
                let mut is_type_alias = false;
                if let Expr::Name(ann_name) = &*node.annotation {
                    if ann_name.id == "TypeAlias" {
                        is_type_alias = true;
                    }
                } else if let Expr::Attribute(ann_attr) = &*node.annotation {
                    if ann_attr.attr.as_str() == "TypeAlias" {
                        is_type_alias = true;
                    }
                }

                if is_type_alias {
                    if let Expr::Name(name_node) = &*node.target {
                        let qualified_name = self.get_qualified_name(&name_node.id);
                        self.add_ref(qualified_name);
                    }
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

                // In ruff, StmtIf uses elif_else_clauses instead of orelse
                for clause in &node.elif_else_clauses {
                    // Each clause has an optional test (None for else) and a body
                    if let Some(test) = &clause.test {
                        self.visit_expr(test);
                    }
                    for stmt in &clause.body {
                        self.visit_stmt(stmt);
                    }
                }
            }
            Stmt::For(node) => {
                self.visit_expr(&node.iter);
                self.visit_definition_target(&node.target);
                for stmt in &node.body {
                    self.visit_stmt(stmt);
                }
                for stmt in &node.orelse {
                    self.visit_stmt(stmt);
                }
            }
            // Note: ruff merges AsyncFor into For with is_async flag, so no separate handling needed
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
            // Note: ruff merges AsyncWith into With with is_async flag, so no separate handling needed
            Stmt::Try(node) => {
                let mut catches_import_error = false;
                for handler in &node.handlers {
                    let ruff_python_ast::ExceptHandler::ExceptHandler(h) = handler;
                    if let Some(type_) = &h.type_ {
                        // Check if it catches ImportError or ModuleNotFoundError
                        if let Expr::Name(name) = &**type_ {
                            if name.id.as_str() == "ImportError"
                                || name.id.as_str() == "ModuleNotFoundError"
                            {
                                catches_import_error = true;
                            }
                        } else if let Expr::Tuple(tuple) = &**type_ {
                            for elt in &tuple.elts {
                                if let Expr::Name(name) = elt {
                                    if name.id.as_str() == "ImportError"
                                        || name.id.as_str() == "ModuleNotFoundError"
                                    {
                                        catches_import_error = true;
                                    }
                                }
                            }
                        }
                    }
                }

                let prev_in_import_error = self.in_import_error_block;
                if catches_import_error {
                    self.in_import_error_block = true;
                }

                for stmt in &node.body {
                    self.visit_stmt(stmt);
                }

                self.in_import_error_block = prev_in_import_error;

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
            // Note: ruff merges TryStar into Try with is_star on handlers, so no separate handling needed
            Stmt::Return(node) => {
                if let Some(value) = &node.value {
                    // Track returned function/class names as used
                    // This handles decorator wrappers like: def decorator(): def wrapper(): ...; return wrapper
                    if let Expr::Name(name_node) = &**value {
                        if name_node.ctx.is_load() {
                            let name = name_node.id.to_string();
                            // Try to resolve to qualified name first
                            // 1. Try to resolve to qualified name first
                            if let Some(qualified) = self.resolve_name(&name) {
                                self.add_ref(qualified);
                            }
                        }
                    }
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
            Stmt::Global(node) => {
                for name in &node.names {
                    // Register global declaration in the current scope
                    if let Some(scope) = self.scope_stack.last_mut() {
                        scope.global_declarations.insert(name.id.to_string());
                    }
                    // Mark as referenced to ensure the global variable is counted as used
                    self.add_ref(name.id.to_string());
                    if !self.module_name.is_empty() {
                        let qualified = format!("{}.{}", self.module_name, name.id);
                        self.add_ref(qualified);
                    }
                }
            }
            _ => {}
        }

        self.depth -= 1;
    }

    // Helper function to handle shared logic between FunctionDef and AsyncFunctionDef
    #[allow(clippy::too_many_lines)]
    fn visit_function_def(
        &mut self,
        name_node: &ruff_python_ast::Identifier,
        decorator_list: &[ruff_python_ast::Decorator],
        parameters: &ruff_python_ast::Parameters,
        body: &[ruff_python_ast::Stmt],
        range: ruff_text_size::TextRange,
    ) {
        let name = name_node.id.as_str();
        let qualified_name = self.get_qualified_name(name);

        // Determine if it's a function or a method based on class stack.
        let def_type = if self.class_stack.is_empty() {
            "function"
        } else {
            "method"
        };

        // Use name identifier for the start position/line (for precise reporting)
        let def_range = name_node.range;
        let start_byte = def_range.start().into();
        let line = self.line_index.line_index(def_range.start());
        let col = self.line_index.column_index(def_range.start());

        // Use full stmt range for the end position/line (to cover the body for analysis)
        let end_byte = range.end().into();
        let end_line = self.line_index.line_index(range.end());

        self.add_definition(DefinitionInfo {
            name: qualified_name.clone(),
            def_type: def_type.to_owned(),
            line,
            end_line,
            col,
            start_byte,
            end_byte,
            full_start_byte: range.start().into(),
            base_classes: SmallVec::new(),
        });

        // Heuristic: If method raises NotImplementedError, treat as abstract/interface (confidence 0)
        let raises_not_implemented = body.iter().any(|s| {
            if let ruff_python_ast::Stmt::Raise(r) = s {
                if let Some(exc) = &r.exc {
                    match &**exc {
                        ruff_python_ast::Expr::Name(n) => return n.id == "NotImplementedError",
                        ruff_python_ast::Expr::Call(c) => {
                            if let ruff_python_ast::Expr::Name(n) = &*c.func {
                                return n.id == "NotImplementedError";
                            }
                        }
                        _ => {}
                    }
                }
            }
            false
        });

        if raises_not_implemented {
            if let Some(last_def) = self.definitions.last_mut() {
                last_def.confidence = 0;
            }
        }

        // Collection Logic for ABC and Protocols
        if let Some(class_name) = self.class_stack.last() {
            // 1. Collect Abstract Methods
            if let Some(true) = self.abc_class_stack.last() {
                let is_abstract = decorator_list.iter().any(|d| {
                    let expr = match &d.expression {
                        ruff_python_ast::Expr::Call(call) => &*call.func,
                        _ => &d.expression,
                    };
                    match expr {
                        ruff_python_ast::Expr::Name(n) => n.id == "abstractmethod",
                        ruff_python_ast::Expr::Attribute(attr) => {
                            attr.attr.as_str() == "abstractmethod"
                        }
                        _ => false,
                    }
                });

                if is_abstract {
                    self.abc_abstract_methods
                        .entry(class_name.clone())
                        .or_default()
                        .insert(name.to_owned());

                    // Mark abstract method as "used" (confidence 0)
                    if let Some(def) = self.definitions.last_mut() {
                        def.confidence = 0;
                    }
                }
            }

            // 2. Collect Protocol Methods
            if let Some(true) = self.protocol_class_stack.last() {
                self.protocol_methods
                    .entry(class_name.clone())
                    .or_default()
                    .insert(name.to_owned());

                // Note: We do NOT strictly ignoring Protocol methods here because
                // sometimes they might be reported as dead in benchmarks.
                // We rely on duck typing usage to save them if used.
                // Or maybe we should?
                // Reverting confidence=0 to fix regression.
            }
        }

        // Register the function in the current (parent) scope's local_var_map
        // Register the function in the current (parent) scope's local_var_map
        // This allows nested function calls like `used_inner()` to be resolved
        // when the call happens in the parent scope.
        self.add_local_def(name.to_owned(), qualified_name.clone());

        // Enter function scope
        self.enter_scope(ScopeType::Function(CompactString::from(name)));

        // Framework detection logic
        let mut should_add_ref = false;
        if let Some(scope) = self.scope_stack.last_mut() {
            for decorator in decorator_list {
                // Handle both @app.route and @app.route(...)
                let expr = match &decorator.expression {
                    ruff_python_ast::Expr::Call(call) => &*call.func,
                    _ => &decorator.expression,
                };

                if let ruff_python_ast::Expr::Attribute(attr) = expr {
                    if let ruff_python_ast::Expr::Name(name) = &*attr.value {
                        let base = name.id.as_str();
                        // Common framework patterns: app.route, router.get, celery.task
                        if matches!(base, "app" | "router" | "celery") {
                            scope.is_framework = true;
                            // Check if this is an Azure Function (special case for 1:1 mapping)
                            if base == "app" {
                                // Update definition to mark as framework managed and used
                                if let Some(def) = self.definitions.last_mut() {
                                    def.is_framework_managed = true;
                                    def.is_exported = true; // Treat as entry point
                                    if def.references == 0 {
                                        def.references = 1;
                                    }
                                }
                                should_add_ref = true;
                            } else {
                                // Other frameworks
                                if let Some(def) = self.definitions.last_mut() {
                                    def.is_framework_managed = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        if should_add_ref {
            self.add_ref(qualified_name.clone());
        }

        // Check if we should skip parameter tracking (Abstract methods, Protocols, Overloads)
        let mut skip_parameters = false;

        // 1. Check if inside a Protocol class
        if let Some(true) = self.protocol_class_stack.last() {
            skip_parameters = true;
        }

        // 2. Check for @abstractmethod or @overload decorators
        if !skip_parameters {
            for decorator in decorator_list {
                let expr = match &decorator.expression {
                    ruff_python_ast::Expr::Call(call) => &*call.func,
                    _ => &decorator.expression,
                };

                if let ruff_python_ast::Expr::Name(name) = expr {
                    if name.id == "abstractmethod" || name.id == "overload" {
                        skip_parameters = true;
                        break;
                    }
                } else if let ruff_python_ast::Expr::Attribute(attr) = expr {
                    if attr.attr.as_str() == "abstractmethod" || attr.attr.as_str() == "overload" {
                        skip_parameters = true;
                        break;
                    }
                }
            }
        }

        // Track parameters
        let mut param_names = FxHashSet::default();

        // Helper to extract parameter name (ruff uses ParameterWithDefault with .parameter.name)
        let extract_param_name =
            |arg: &ast::ParameterWithDefault| -> String { arg.parameter.name.to_string() };

        // Positional-only parameters
        for arg in &parameters.posonlyargs {
            let param_name = extract_param_name(arg);
            param_names.insert(param_name.clone());
            let param_qualified = if param_name != "self" && param_name != "cls" {
                format!("{qualified_name}.{param_name}")
            } else {
                param_name.clone()
            };
            self.add_local_def(param_name.clone(), param_qualified.clone());

            // Skip self and cls - they're implicit
            // Also skip if we are in an abstract method or protocol
            if !skip_parameters && param_name != "self" && param_name != "cls" {
                let (p_line, p_end_line, p_col, p_start_byte, p_end_byte) =
                    self.get_range_info(arg);
                self.add_definition(DefinitionInfo {
                    name: param_qualified,
                    def_type: "parameter".to_owned(),
                    line: p_line,
                    end_line: p_end_line,
                    col: p_col,
                    start_byte: p_start_byte,
                    end_byte: p_end_byte,
                    full_start_byte: p_start_byte,
                    base_classes: SmallVec::new(),
                });
            }
        }

        // Regular positional parameters
        for arg in &parameters.args {
            let param_name = extract_param_name(arg);
            param_names.insert(param_name.clone());
            let param_qualified = if param_name != "self" && param_name != "cls" {
                format!("{qualified_name}.{param_name}")
            } else {
                param_name.clone()
            };
            self.add_local_def(param_name.clone(), param_qualified.clone());

            // Skip self and cls
            // Also skip if we are in an abstract method or protocol
            if !skip_parameters && param_name != "self" && param_name != "cls" {
                let (p_line, p_end_line, p_col, p_start_byte, p_end_byte) =
                    self.get_range_info(arg);
                self.add_definition(DefinitionInfo {
                    name: param_qualified,
                    def_type: "parameter".to_owned(),
                    line: p_line,
                    end_line: p_end_line,
                    col: p_col,
                    start_byte: p_start_byte,
                    end_byte: p_end_byte,
                    full_start_byte: p_start_byte,
                    base_classes: SmallVec::new(),
                });
            }
        }

        // Keyword-only parameters
        for arg in &parameters.kwonlyargs {
            let param_name = extract_param_name(arg);
            param_names.insert(param_name.clone());
            let param_qualified = format!("{qualified_name}.{param_name}");
            self.add_local_def(param_name.clone(), param_qualified.clone());
            let (p_line, p_end_line, p_col, p_start_byte, p_end_byte) = self.get_range_info(arg);

            if !skip_parameters {
                self.add_definition(DefinitionInfo {
                    name: param_qualified,
                    def_type: "parameter".to_owned(),
                    line: p_line,
                    end_line: p_end_line,
                    col: p_col,
                    start_byte: p_start_byte,
                    end_byte: p_end_byte,
                    full_start_byte: p_start_byte,
                    base_classes: smallvec::SmallVec::new(),
                });
            }
        }

        // *args parameter (ruff uses .name instead of .arg)
        if let Some(vararg) = &parameters.vararg {
            let param_name = vararg.name.to_string();
            param_names.insert(param_name.clone());
            let param_qualified = format!("{qualified_name}.{param_name}");
            self.add_local_def(param_name, param_qualified.clone());
            let (p_line, p_end_line, p_col, p_start_byte, p_end_byte) =
                self.get_range_info(&**vararg);

            if !skip_parameters {
                self.add_definition(DefinitionInfo {
                    name: param_qualified,
                    def_type: "parameter".to_owned(),
                    line: p_line,
                    end_line: p_end_line,
                    col: p_col,
                    start_byte: p_start_byte,
                    end_byte: p_end_byte,
                    full_start_byte: p_start_byte,
                    base_classes: smallvec::SmallVec::new(),
                });
            }
        }

        // **kwargs parameter (ruff uses .name instead of .arg)
        if let Some(kwarg) = &parameters.kwarg {
            let param_name = kwarg.name.to_string();
            param_names.insert(param_name.clone());
            let param_qualified = format!("{qualified_name}.{param_name}");
            self.add_local_def(param_name, param_qualified.clone());
            let (p_line, p_end_line, p_col, p_start_byte, p_end_byte) =
                self.get_range_info(&**kwarg);

            if !skip_parameters {
                self.add_definition(DefinitionInfo {
                    name: param_qualified,
                    def_type: "parameter".to_owned(),
                    line: p_line,
                    end_line: p_end_line,
                    col: p_col,
                    start_byte: p_start_byte,
                    end_byte: p_end_byte,
                    full_start_byte: p_start_byte,
                    base_classes: smallvec::SmallVec::new(),
                });
            }
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
    ///
    /// This function tracks name references and attribute accesses to determine
    /// which symbols are actually used in the code.
    ///
    /// # Structure
    ///
    /// The function handles these expression types:
    /// - **Name** (line ~1259): Variable/name references with scope resolution
    /// - **Attribute** (line ~1300+): Attribute access (obj.attr)
    /// - **Call** (line ~1350+): Function/method calls with argument tracking
    /// - **Subscript** (line ~1400+): Index operations (`obj[key]`)
    /// - **Lambda/Comprehensions** (line ~1450+): Nested scopes
    /// - **Literals/Operators** (various): `BinOp`, `Compare`, `BoolOp`, etc.
    ///
    /// # Recursion Safety
    ///
    /// Uses `MAX_RECURSION_DEPTH` guard shared with `visit_stmt`.
    fn visit_name_expr(&mut self, node: &ast::ExprName) {
        if node.ctx.is_load() {
            let name = node.id.to_string();

            // Try to resolve using scope stack first
            if let Some((qualified, scope_idx)) = self.resolve_name_with_info(&name) {
                // Mark as captured if found in a parent scope
                if scope_idx < self.scope_stack.len() - 1 {
                    self.captured_definitions.insert(qualified.clone());
                }
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

    fn visit_call_expr(&mut self, node: &ast::ExprCall) {
        // Check for dynamic execution or reflection
        if let Expr::Name(func_name) = &*node.func {
            let name = func_name.id.as_str();
            if name == "eval" {
                // Optimization: If eval is called with a string literal, parse it for name references
                // instead of marking the whole scope as dynamic.
                let mut handled_as_literal = false;
                if let Some(Expr::StringLiteral(s)) = node.arguments.args.first() {
                    // Extract identifiers from the string
                    // We construct the Regex locally. Since this is only for eval(), the per-call cost is verified acceptable.
                    if let Ok(re) = Regex::new(r"\b[a-zA-Z_]\w*\b") {
                        // s.value is a StringLiteralValue, convert to string
                        let val = s.value.to_string();
                        for m in re.find_iter(&val) {
                            self.add_ref(m.as_str().to_owned());
                        }
                        handled_as_literal = true;
                    }
                }

                if !handled_as_literal {
                    let scope_id = self.get_current_scope_id();
                    self.dynamic_scopes.insert(scope_id);
                }
            } else if name == "exec" || name == "globals" || name == "locals" {
                let scope_id = self.get_current_scope_id();
                self.dynamic_scopes.insert(scope_id);
            }

            // Special handling for hasattr(obj, "attr") to detect attribute usage
            if name == "hasattr" && node.arguments.args.len() == 2 {
                // Extract the object (first arg) and attribute name (second arg)
                if let (Expr::Name(obj_name), Expr::StringLiteral(attr_str)) =
                    (&node.arguments.args[0], &node.arguments.args[1])
                {
                    let attr_value = attr_str.value.to_string();
                    // Construct the qualified attribute name
                    // e.g., hasattr(Colors, "GREEN") -> Colors.GREEN
                    let attr_ref = format!("{}.{}", obj_name.id, attr_value);
                    self.add_ref(attr_ref);

                    // Also try with module prefix
                    if !self.module_name.is_empty() {
                        let full_attr_ref =
                            format!("{}.{}.{}", self.module_name, obj_name.id, attr_value);
                        self.add_ref(full_attr_ref);
                    }
                }
            }
        }

        self.visit_expr(&node.func);
        for arg in &node.arguments.args {
            self.visit_expr(arg);
        }
        // Don't forget keyword arguments (e.g., func(a=b))
        for keyword in &node.arguments.keywords {
            self.visit_expr(&keyword.value);
        }
    }

    fn visit_attribute_expr(&mut self, node: &ast::ExprAttribute) {
        // Track attribute access strictly as attribute reference (prefixed with dot)
        // This distinguishes `d.keys()` (attribute) from `keys` (variable)
        self.add_ref(format!(".{}", node.attr));

        // Check for self-referential method call (recursive method)
        // If we see self.method_name() and method_name matches current function in function_stack
        if let Expr::Name(base_node) = &*node.value {
            if base_node.id.as_str() == "self" || base_node.id.as_str() == "cls" {
                let attr_name = node.attr.as_str();
                // Check if this is a call to the current method (recursive)
                if let Some(current_method_qualified) = self.function_stack.last() {
                    // Extract simple name from qualified name stored in stack
                    let current_method_simple =
                        if let Some(idx) = current_method_qualified.rfind('.') {
                            &current_method_qualified[idx + 1..]
                        } else {
                            current_method_qualified.as_str()
                        };

                    if current_method_simple == attr_name {
                        // This is a self-referential call
                        self.self_referential_methods
                            .insert(current_method_qualified.clone());
                    }
                }
            }
        }

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

    fn visit_string_literal(&mut self, node: &ast::ExprStringLiteral) {
        let s = node.value.to_string();
        // Heuristic: If a string looks like a simple identifier or dotted path (no spaces),
        // track it as a reference. This helps with getattr(self, "visit_" + name)
        // and stringified type hints like "models.User".
        if !s.contains(' ') && !s.is_empty() {
            self.add_ref(s.clone());

            // Enhanced: Extract type names from string type annotations
            // Handles patterns like "List[Dict[str, int]]", "Optional[User]"
            // Extract alphanumeric identifiers (type names)
            let mut current_word = String::new();
            for ch in s.chars() {
                if ch.is_alphanumeric() || ch == '_' {
                    current_word.push(ch);
                } else if !current_word.is_empty() {
                    // Check if it looks like a type name (starts with uppercase)
                    if current_word.chars().next().is_some_and(char::is_uppercase) {
                        self.add_ref(current_word.clone());
                    }
                    current_word.clear();
                }
            }
            // Don't forget the last word
            if !current_word.is_empty()
                && current_word.chars().next().is_some_and(char::is_uppercase)
            {
                self.add_ref(current_word);
            }
        }
    }

    /// Visits an expression node.
    ///
    /// The function handles these expression types:
    /// - **Name**: Variable/name references with scope resolution
    /// - **Attribute**: Attribute access (obj.attr)
    /// - **Call**: Function/method calls with argument tracking
    /// - **Subscript**: Index operations (`obj[key]`)
    /// - **Lambda/Comprehensions**: Nested scopes
    /// - **Literals/Operators**: `BinOp`, `Compare`, `BoolOp`, etc.
    ///
    /// # Recursion Safety
    ///
    /// Uses `MAX_RECURSION_DEPTH` guard shared with `visit_stmt`.
    #[allow(clippy::too_many_lines)]
    pub fn visit_expr(&mut self, expr: &Expr) {
        // Recursion depth guard to prevent stack overflow on deeply nested code
        if self.depth >= MAX_RECURSION_DEPTH {
            self.recursion_limit_hit = true;
            return;
        }
        self.depth += 1;

        match expr {
            // Name usage (variable access)
            Expr::Name(node) => self.visit_name_expr(node),
            // Function call
            Expr::Call(node) => self.visit_call_expr(node),
            // Attribute access (e.g., obj.attr)
            Expr::Attribute(node) => self.visit_attribute_expr(node),
            // Dynamic Dispatch / String References
            Expr::StringLiteral(node) => self.visit_string_literal(node),

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
            Expr::If(node) => {
                self.visit_expr(&node.test);
                self.visit_expr(&node.body);
                self.visit_expr(&node.orelse);
            }
            Expr::Dict(node) => {
                for item in &node.items {
                    if let Some(k) = &item.key {
                        self.visit_expr(k);
                    }
                    self.visit_expr(&item.value);
                }
            }
            Expr::Set(node) => {
                for elt in &node.elts {
                    self.visit_expr(elt);
                }
            }
            Expr::ListComp(node) => {
                for gen in &node.generators {
                    self.visit_expr(&gen.iter);
                    self.visit_definition_target(&gen.target);
                    for if_expr in &gen.ifs {
                        self.visit_expr(if_expr);
                    }
                }
                self.visit_expr(&node.elt);
            }
            Expr::SetComp(node) => {
                for gen in &node.generators {
                    self.visit_expr(&gen.iter);
                    self.visit_definition_target(&gen.target);
                    for if_expr in &gen.ifs {
                        self.visit_expr(if_expr);
                    }
                }
                self.visit_expr(&node.elt);
            }
            Expr::DictComp(node) => {
                for gen in &node.generators {
                    self.visit_expr(&gen.iter);
                    self.visit_definition_target(&gen.target);
                    for if_expr in &gen.ifs {
                        self.visit_expr(if_expr);
                    }
                }
                self.visit_expr(&node.key);
                self.visit_expr(&node.value);
            }
            Expr::Generator(node) => {
                for gen in &node.generators {
                    self.visit_expr(&gen.iter);
                    self.visit_definition_target(&gen.target);
                    for if_expr in &gen.ifs {
                        self.visit_expr(if_expr);
                    }
                }
                self.visit_expr(&node.elt);
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
            Expr::FString(node) => {
                for part in &node.value {
                    match part {
                        ast::FStringPart::Literal(_) => {}
                        ast::FStringPart::FString(f) => {
                            // Visit expressions inside f-string interpolations
                            for element in &f.elements {
                                if let ast::InterpolatedStringElement::Interpolation(interp) =
                                    element
                                {
                                    self.visit_expr(&interp.expression);
                                }
                            }
                        }
                    }
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

        self.depth -= 1;
    }

    /// Visits a definition target (LHS of assignment or loop variable).
    /// Registers variables as definitions.
    fn visit_definition_target(&mut self, target: &Expr) {
        match target {
            Expr::Name(node) => {
                let name = node.id.to_string();
                let qualified_name = self.get_qualified_name(&name);
                let (line, end_line, col, start_byte, end_byte) = self.get_range_info(node);

                self.add_definition(DefinitionInfo {
                    name: qualified_name.clone(),
                    def_type: "variable".to_owned(),
                    line,
                    end_line,
                    col,
                    start_byte,
                    end_byte,
                    full_start_byte: start_byte,
                    base_classes: smallvec::SmallVec::new(),
                });
                self.add_local_def(name, qualified_name);
            }
            Expr::Tuple(node) => {
                for elt in &node.elts {
                    self.visit_definition_target(elt);
                }
            }
            Expr::List(node) => {
                for elt in &node.elts {
                    self.visit_definition_target(elt);
                }
            }
            Expr::Starred(node) => {
                self.visit_definition_target(&node.value);
            }
            // Use visits for attribute/subscript to ensure we track usage of the object/index
            Expr::Attribute(node) => {
                self.visit_expr(&node.value);
            }
            Expr::Subscript(node) => {
                self.visit_expr(&node.value);
                self.visit_expr(&node.slice);
            }
            _ => {}
        }
    }

    /// Helper to recursively visit match patterns
    fn visit_match_pattern(&mut self, pattern: &ast::Pattern) {
        // Recursion depth guard to prevent stack overflow on deeply nested code
        if self.depth >= MAX_RECURSION_DEPTH {
            self.recursion_limit_hit = true;
            return;
        }
        self.depth += 1;

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
                    // Assuming rest identifier has range, we use node range as approximation if not available
                    // Actually rest is an Identifier which might not be Ranged directly in some AST versions,
                    // but usually String/Identifier is just string.
                    // Wait, `rest` is Identifier. In ruff_python_ast Identifier wraps string and range.
                    // But looking at code `if let Some(rest) = &node.rest`, rest type is `Identifier`.
                    // Does Identifier impl Ranged? Yes.
                    let (line, end_line, col, start_byte, end_byte) = self.get_range_info(node);
                    // Using node range because rest match captures the rest
                    self.add_definition(DefinitionInfo {
                        name: qualified_name.clone(),
                        def_type: "variable".to_owned(),
                        line,
                        end_line,
                        col,
                        start_byte,
                        end_byte,
                        full_start_byte: start_byte,
                        base_classes: smallvec::SmallVec::new(),
                    });
                    // Add to local scope so it can be resolved when used
                    self.add_local_def(rest.to_string(), qualified_name);
                }
            }
            ast::Pattern::MatchClass(node) => {
                self.visit_expr(&node.cls);
                for p in &node.arguments.patterns {
                    self.visit_match_pattern(p);
                }
                for k in &node.arguments.keywords {
                    self.visit_match_pattern(&k.pattern);
                }
            }
            ast::Pattern::MatchStar(node) => {
                if let Some(name) = &node.name {
                    let qualified_name = self.get_qualified_name(name);
                    let (line, end_line, col, start_byte, end_byte) = self.get_range_info(node);
                    self.add_definition(DefinitionInfo {
                        name: qualified_name.clone(),
                        def_type: "variable".to_owned(),
                        line,
                        end_line,
                        col,
                        start_byte,
                        end_byte,
                        full_start_byte: start_byte,
                        base_classes: smallvec::SmallVec::new(),
                    });
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
                    let (line, end_line, col, start_byte, end_byte) = self.get_range_info(node);
                    self.add_definition(DefinitionInfo {
                        name: qualified_name.clone(),
                        def_type: "variable".to_owned(),
                        line,
                        end_line,
                        col,
                        start_byte,
                        end_byte,
                        full_start_byte: start_byte,
                        base_classes: smallvec::SmallVec::new(),
                    });
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

        self.depth -= 1;
    }
}
