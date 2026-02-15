use super::*;

/// Serialize Arc<PathBuf> as a plain `PathBuf` for JSON output
pub(super) fn serialize_arc_path<S>(path: &Arc<PathBuf>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    path.as_ref().serialize(serializer)
}

/// Deserialize a plain `PathBuf` into Arc<PathBuf>
pub(super) fn deserialize_arc_path<'de, D>(deserializer: D) -> Result<Arc<PathBuf>, D::Error>
where
    D: Deserializer<'de>,
{
    PathBuf::deserialize(deserializer).map(Arc::new)
}

/// Serialize `SmallVec`<[String; 2]> as a plain Vec<String> for JSON output
pub(super) fn serialize_smallvec_string<S>(
    vec: &SmallVec<[String; 2]>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    vec.as_slice().serialize(serializer)
}

/// Deserialize a plain Vec<String> into `SmallVec`<[String; 2]>
pub(super) fn deserialize_smallvec_string<'de, D>(
    deserializer: D,
) -> Result<SmallVec<[String; 2]>, D::Error>
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
/// Categorization of unused symbols by confidence levels.
pub enum UnusedCategory {
    /// High confidence (90-100) that this is unused.
    #[default]
    DefinitelyUnused,
    /// Moderate confidence (60-89).
    ProbablyUnused,
    /// Low confidence (40-59), possibly intentional (e.g. API stub).
    PossiblyIntentional,
    /// A configuration-shaped constant that appears unused.
    ConfigurationConstant,
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
    /// The confidence category, derived from the score and other factors.
    #[serde(default)]
    pub category: UnusedCategory,
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
    /// Whether this definition is unreachable from entry points.
    #[serde(default)]
    pub is_unreachable: bool,
}

// apply_penalties method removed as it was redundant with heuristics.rs
