use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::Path;

/// Default configuration for specific tools
const DEFAULT_CONFIG: &str = r#"
[cytoscnpy]

# ── Core ──────────────────────────────────────────────────────────────────────

# Minimum confidence score (0-100) a finding must reach before it is reported.
# Lower values surface more findings; higher values reduce noise.
# confidence = 60

# Scan for hard-coded secrets and high-entropy strings (API keys, tokens, etc.).
# secrets = true

# Scan for dangerous code patterns: SQL injection, XSS, command injection, etc.
# danger = true

# Report code-quality issues: high complexity, deep nesting, long functions, etc.
# quality = true

# Include pytest/unittest test files in the analysis.
# include_tests = false

# Include Jupyter notebook (.ipynb) cells in the analysis.
# include_ipynb = false

# How the project is used externally.
# "library"     – public symbols are assumed to be part of the API (fewer false positives).
# "application" – reduces public-API assumptions; stricter dead-code detection.
# project_type = "library"


# ── Quality thresholds ────────────────────────────────────────────────────────

# McCabe cyclomatic complexity limit per function. Exceeding this is a finding.
# max_complexity = 10

# Maximum nesting depth (if/for/while/try) before a finding is emitted.
# max_nesting = 3

# Maximum number of parameters a function may have.
# max_args = 5

# Maximum number of lines a function body may span.
# max_lines = 50

# Minimum Maintainability Index (0-100). Functions below this score are flagged.
# min_mi = 40.0


# ── Path filters ──────────────────────────────────────────────────────────────

# Directories to skip entirely during analysis.
# exclude_folders = ["build", "dist", ".venv", ".git", "__pycache__", ".mypy_cache", ".pytest_cache"]

# Directories to force-include even when they are git-ignored.
# include_folders = ["src"]


# ── Rule suppression ──────────────────────────────────────────────────────────

# Rule IDs to silence globally across the entire project.
# ignore = ["CSP-P003"]

# Silence specific rules only for files matching a glob pattern.
# per-file-ignores = { "tests/*" = ["CSP-D701"], "**/__init__.py" = ["CSP-L001"] }


# ── Clone detection ───────────────────────────────────────────────────────────

# Detect Type-1/2/3 duplicate code blocks across the project.
# clones = false

# How similar two blocks must be (0.0-1.0) to be reported as a clone pair.
# clone_similarity = 0.8


# ── CI/CD gate ────────────────────────────────────────────────────────────────

# Exit with code 1 when the percentage of unused definitions exceeds this value.
# fail_threshold = 5.0


# ── Inline whitelist ──────────────────────────────────────────────────────────
# Suppress specific dead-code symbols without a separate whitelist file.
# Each entry can target a single name, a wildcard pattern, or a regex.

# [[cytoscnpy.whitelist]]
# # The symbol name (or pattern) to suppress.
# name = "my_unused_fn"
# # Match mode: "exact" (default), "wildcard" (glob-style), or "regex".
# pattern = "exact"
# # Optional: restrict this entry to files matching a glob.
# # file = "src/api/*.py"


# ── Secrets scanning (advanced) ───────────────────────────────────────────────

# [cytoscnpy.secrets_config]
# # Shannon entropy threshold; strings above this score are flagged.
# entropy_threshold = 4.5
# # Minimum string length before entropy is evaluated.
# min_length = 16
# # Enable the entropy-based scanner (disable to use pattern-only mode).
# entropy_enabled = true
# # Also scan comments and docstrings for secrets.
# scan_comments = true
# # Skip triple-quoted docstrings during secret scanning.
# skip_docstrings = false
# # Combined confidence score threshold (0-100) for a secret finding.
# min_score = 50
# # Extra variable/parameter names that should be treated as suspicious.
# suspicious_names = []

# Add custom regex patterns for secret detection:
# [[cytoscnpy.secrets_config.patterns]]
# # Human-readable name shown in findings.
# name = "My Token"
# # Regular expression to match.
# regex = "mytoken-[0-9a-zA-Z]{32}"
# # Severity level: LOW, MEDIUM, HIGH, or CRITICAL.
# severity = "HIGH"


# ── Danger / taint analysis (advanced) ───────────────────────────────────────

# [cytoscnpy.danger_config]
# # Enable interprocedural taint tracking (data-flow from sources to sinks).
# enable_taint = true
# # Minimum severity to report: LOW, MEDIUM, HIGH, or CRITICAL.
# severity_threshold = "LOW"
# # Rule IDs to exclude from danger scanning.
# excluded_rules = []
# # Fully-qualified function names treated as taint sources.
# custom_sources = ["mylib.get_input"]
# # Fully-qualified function names treated as taint sinks.
# custom_sinks = ["mylib.exec_query"]
# # Functions that sanitize / clear taint on their return value.
# custom_sanitizers = ["mylib.escape"]
"#;

pub const DEFAULT_PYPROJECT_CONFIG: &str = r#"
[tool.cytoscnpy]

# ── Core ──────────────────────────────────────────────────────────────────────

# Minimum confidence score (0-100) a finding must reach before it is reported.
# Lower values surface more findings; higher values reduce noise.
# confidence = 60

# Scan for hard-coded secrets and high-entropy strings (API keys, tokens, etc.).
# secrets = true

# Scan for dangerous code patterns: SQL injection, XSS, command injection, etc.
# danger = true

# Report code-quality issues: high complexity, deep nesting, long functions, etc.
# quality = true

# Include pytest/unittest test files in the analysis.
# include_tests = false

# Include Jupyter notebook (.ipynb) cells in the analysis.
# include_ipynb = false

# How the project is used externally.
# "library"     – public symbols are assumed to be part of the API (fewer false positives).
# "application" – reduces public-API assumptions; stricter dead-code detection.
# project_type = "library"


# ── Quality thresholds ────────────────────────────────────────────────────────

# McCabe cyclomatic complexity limit per function. Exceeding this is a finding.
# max_complexity = 10

# Maximum nesting depth (if/for/while/try) before a finding is emitted.
# max_nesting = 3

# Maximum number of parameters a function may have.
# max_args = 5

# Maximum number of lines a function body may span.
# max_lines = 50

# Minimum Maintainability Index (0-100). Functions below this score are flagged.
# min_mi = 40.0


# ── Path filters ──────────────────────────────────────────────────────────────

# Directories to skip entirely during analysis.
# exclude_folders = ["build", "dist", ".venv", ".git", "__pycache__", ".mypy_cache", ".pytest_cache"]

# Directories to force-include even when they are git-ignored.
# include_folders = ["src"]


# ── Rule suppression ──────────────────────────────────────────────────────────

# Rule IDs to silence globally across the entire project.
# ignore = ["CSP-P003"]

# Silence specific rules only for files matching a glob pattern.
# per-file-ignores = { "tests/*" = ["CSP-D701"], "**/__init__.py" = ["CSP-L001"] }


# ── Clone detection ───────────────────────────────────────────────────────────

# Detect Type-1/2/3 duplicate code blocks across the project.
# clones = false

# How similar two blocks must be (0.0-1.0) to be reported as a clone pair.
# clone_similarity = 0.8


# ── CI/CD gate ────────────────────────────────────────────────────────────────

# Exit with code 1 when the percentage of unused definitions exceeds this value.
# fail_threshold = 5.0


# ── Inline whitelist ──────────────────────────────────────────────────────────
# Suppress specific dead-code symbols without a separate whitelist file.
# Each entry can target a single name, a wildcard pattern, or a regex.

# [[tool.cytoscnpy.whitelist]]
# # The symbol name (or pattern) to suppress.
# name = "my_unused_fn"
# # Match mode: "exact" (default), "wildcard" (glob-style), or "regex".
# pattern = "exact"
# # Optional: restrict this entry to files matching a glob.
# # file = "src/api/*.py"


# ── Secrets scanning (advanced) ───────────────────────────────────────────────

# [tool.cytoscnpy.secrets_config]
# # Shannon entropy threshold; strings above this score are flagged.
# entropy_threshold = 4.5
# # Minimum string length before entropy is evaluated.
# min_length = 16
# # Enable the entropy-based scanner (disable to use pattern-only mode).
# entropy_enabled = true
# # Also scan comments and docstrings for secrets.
# scan_comments = true
# # Skip triple-quoted docstrings during secret scanning.
# skip_docstrings = false
# # Combined confidence score threshold (0-100) for a secret finding.
# min_score = 50
# # Extra variable/parameter names that should be treated as suspicious.
# suspicious_names = []

# Add custom regex patterns for secret detection:
# [[tool.cytoscnpy.secrets_config.patterns]]
# # Human-readable name shown in findings.
# name = "My Token"
# # Regular expression to match.
# regex = "mytoken-[0-9a-zA-Z]{32}"
# # Severity level: LOW, MEDIUM, HIGH, or CRITICAL.
# severity = "HIGH"


# ── Danger / taint analysis (advanced) ───────────────────────────────────────

# [tool.cytoscnpy.danger_config]
# # Enable interprocedural taint tracking (data-flow from sources to sinks).
# enable_taint = true
# # Minimum severity to report: LOW, MEDIUM, HIGH, or CRITICAL.
# severity_threshold = "LOW"
# # Rule IDs to exclude from danger scanning.
# excluded_rules = []
# # Fully-qualified function names treated as taint sources.
# custom_sources = ["mylib.get_input"]
# # Fully-qualified function names treated as taint sinks.
# custom_sinks = ["mylib.exec_query"]
# # Functions that sanitize / clear taint on their return value.
# custom_sanitizers = ["mylib.escape"]
"#;

/// Run the init command to initialize CytoScnPy configuration.
/// Executes the init command.
///
/// This creates or updates configuration files in the current directory.
///
/// # Errors
///
/// Returns an error if the current directory cannot be determined or if writing to the configuration file fails.
pub fn run_init<W: Write>(writer: &mut W) -> Result<()> {
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;
    run_init_in(&current_dir, writer)
}

/// Executes the init command in a specific directory.
///
/// This is primarily used for testing.
///
/// # Errors
///
/// Returns an error if writing to the configuration file or .gitignore fails.
pub fn run_init_in<W: Write>(root: &Path, writer: &mut W) -> Result<()> {
    writeln!(writer, "Initializing CytoScnPy configuration...")?;

    handle_config_file(root, writer)?;
    handle_gitignore(root, writer)?;

    writeln!(writer, "Initialization complete!")?;
    Ok(())
}

fn handle_config_file<W: Write>(root: &Path, writer: &mut W) -> Result<()> {
    let pyproject_path = root.join("pyproject.toml");
    let cytoscnpy_toml_path = root.join(".cytoscnpy.toml");

    // 1. Check if .cytoscnpy.toml already exists (highest priority)
    if cytoscnpy_toml_path.exists() {
        writeln!(writer, "  • .cytoscnpy.toml already exists - skipping.")?;
        return Ok(());
    }

    // 2. Check if pyproject.toml already contains the section
    if pyproject_path.exists() {
        let content = fs::read_to_string(&pyproject_path)?;
        if content.contains("[tool.cytoscnpy]") {
            writeln!(
                writer,
                "  - pyproject.toml already contains [tool.cytoscnpy] - skipping."
            )?;
            return Ok(());
        }

        // 3. pyproject.toml exists but no [tool.cytoscnpy]: Append to it
        let mut file = fs::OpenOptions::new().append(true).open(&pyproject_path)?;

        // Add a newline before appending if the file doesn't end with one
        if !content.ends_with('\n') {
            writeln!(file)?;
        }

        writeln!(file, "\n{}", DEFAULT_PYPROJECT_CONFIG.trim())?;
        writeln!(writer, "  - Added default configuration to pyproject.toml.")?;
    } else {
        // 4. Neither exists: Create .cytoscnpy.toml
        let mut file = fs::File::create(&cytoscnpy_toml_path)?;
        writeln!(file, "{}", DEFAULT_CONFIG.trim())?;
        writeln!(
            writer,
            "  - Created .cytoscnpy.toml with default configuration."
        )?;
    }

    Ok(())
}

fn handle_gitignore<W: Write>(root: &Path, writer: &mut W) -> Result<()> {
    let gitignore_path = root.join(".gitignore");
    let ignore_entry = ".cytoscnpy";

    if gitignore_path.exists() {
        let content = fs::read_to_string(&gitignore_path)?;
        // Simple check if the entry exists
        // Note: This isn't a robust .gitignore parser, but sufficient for simple cases
        if content.contains(ignore_entry) {
            writeln!(
                writer,
                "  - .gitignore already contains {ignore_entry} - skipping."
            )?;
        } else {
            let mut file = fs::OpenOptions::new().append(true).open(&gitignore_path)?;

            // Add a newline before appending if the file doesn't end with one
            if !content.ends_with('\n') && !content.is_empty() {
                writeln!(file)?;
            }

            writeln!(file, "{ignore_entry}")?;
            writeln!(writer, "  • Added {ignore_entry} to .gitignore.")?;
        }
    } else {
        let mut file = fs::File::create(&gitignore_path)?;
        writeln!(file, "{ignore_entry}")?;
        writeln!(writer, "  • Created .gitignore with {ignore_entry}.")?;
    }

    Ok(())
}
