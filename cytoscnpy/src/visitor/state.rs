#![allow(missing_docs)]

use super::*;

pub struct CytoScnPyVisitor<'a> {
    pub definitions: Vec<Definition>,
    /// Collected reference counts (name -> count). `PathBuf` removed as it was never used.
    pub references: FxHashMap<String, usize>,
    /// Names explicitly exported via `__all__`.
    pub exports: Vec<String>,
    /// Dynamic imports detected.
    pub dynamic_imports: Vec<String>,
    /// Project type controls public API export assumptions.
    pub(super) project_type: crate::config::ProjectType,
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
    /// Import binding graph: local qualified import symbol -> source qualified symbol.
    /// Used in aggregation to propagate references across re-export chains.
    pub import_bindings: FxHashMap<String, String>,
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
    pub(super) cached_scope_prefix: String,
    /// Current recursion depth for ``visit_stmt`/`visit_expr`` to prevent stack overflow.
    pub(super) depth: usize,
    /// Whether the recursion limit was hit during traversal.
    pub recursion_limit_hit: bool,
    /// Set of names that are automatically called by frameworks (e.g., `main`, `setup`, `teardown`).
    pub(super) auto_called: FxHashSet<&'static str>,
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
