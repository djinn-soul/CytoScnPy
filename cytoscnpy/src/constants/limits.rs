/// Maximum recursion depth for AST visitor to prevent stack overflow on deeply nested code.
pub const MAX_RECURSION_DEPTH: usize = 400;
/// Number of files to process per chunk in parallel processing.
pub const CHUNK_SIZE: usize = 500;
/// Minimum number of lines for a code block to be considered a clone candidate.
pub const MIN_CLONE_LINES: usize = 4;
/// Maximum number of occurrences of a structural pattern before it is treated as boilerplate.
pub const BOILERPLATE_THRESHOLD: usize = 1000;
/// Default configuration filename.
pub const CONFIG_FILENAME: &str = ".cytoscnpy.toml";
/// Python project configuration filename.
pub const PYPROJECT_FILENAME: &str = "pyproject.toml";
/// Rule ID for configuration-related errors.
pub const RULE_ID_CONFIG_ERROR: &str = "CSP-CONFIG-ERROR";
/// Default value for whether taint analysis is enabled when not explicitly configured.
pub const TAINT_ENABLED_DEFAULT: bool = true;
