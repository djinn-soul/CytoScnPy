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
/// Internal default threshold for LCOM4 cohesion checks.
pub const QUALITY_COHESION_LCOM4_THRESHOLD: usize = 6;
/// Below this connected-component size, clone grouping skips the density
/// check (too cheap to matter, and small components are usually fully
/// connected anyway).
pub const CLONE_GROUP_SPLIT_MIN_SIZE: usize = 4;
/// Minimum fraction of all-pairs edges (relative to a fully connected
/// component) required to keep a clone component as a single group.
/// Below this, the component is split into tighter near-cliques to avoid
/// reporting transitively-chained, mutually-dissimilar instances as one
/// clone group (A~B~C reported as clones even though A and C never matched).
pub const CLONE_GROUP_MIN_DENSITY: f64 = 0.5;
/// Above this component size, skip near-clique splitting entirely and keep
/// the union-find component as a single group. Greedy near-clique growth is
/// worst-case O(n^2) (a sparse "star" component pays the most); on a
/// pathological component this bounds the cost instead of letting it grow
/// unbounded. Components this large are rare in practice — this only trades
/// a little precision on an extreme outlier for a hard time ceiling.
pub const CLONE_GROUP_SPLIT_MAX_SIZE: usize = 300;
