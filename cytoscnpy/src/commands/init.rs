use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::Path;

/// Default configuration for specific tools
const DEFAULT_CONFIG: &str = r#"
[cytoscnpy]
# Core settings
confidence = 60            # Confidence threshold (0-100)
secrets = true             # Enable secrets scanning
danger = true              # Enable dangerous code scanning
quality = true             # Enable quality checks
include_tests = false      # Include test files in analysis
include_ipynb = false      # Include Jupyter notebooks

# Quality thresholds
max_complexity = 10        # Max cyclomatic complexity
max_nesting = 3            # Max nesting depth
max_args = 5               # Max function arguments
max_lines = 50             # Max function lines
min_mi = 40.0              # Min Maintainability Index

# Path filters
exclude_folders = ["build", "dist", ".venv", ".git", "__pycache__", ".mypy_cache", ".pytest_cache"]
include_folders = ["src"]  # Force-include these folders even if ignored by git

# CI/CD
fail_threshold = 5.0       # Exit 1 if >N% unused code
"#;

pub const DEFAULT_PYPROJECT_CONFIG: &str = r#"
[tool.cytoscnpy]
# Core settings
confidence = 60            # Confidence threshold (0-100)
secrets = true             # Enable secrets scanning
danger = true              # Enable dangerous code scanning
quality = true             # Enable quality checks
max_args = 5               # Max function arguments
max_lines = 50             # Max function lines
max_complexity = 10        # Max cyclomatic complexity
max_nesting = 3            # Max nesting depth
min_mi = 40.0              # Min Maintainability Index

# Path filters
exclude_folders = ["build", "dist", ".venv", ".git", "__pycache__", ".mypy_cache", ".pytest_cache"]
include_folders = ["src"]  # Force-include these folders even if ignored by git
include_tests = false      # Include test files in analysis
include_ipynb = false      # Include Jupyter notebooks

# CI/CD
fail_threshold = 5.0       # Exit 1 if >N% unused code
"#;

/// Run the init command to initialize CytoScnPy configuration.
/// Executes the init command.
///
/// This creates or updates configuration files in the current directory.
pub fn run_init<W: Write>(writer: &mut W) -> Result<()> {
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;
    run_init_in(&current_dir, writer)
}

/// Executes the init command in a specific directory.
///
/// This is primarily used for testing.
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
                "  • pyproject.toml already contains [tool.cytoscnpy] - skipping."
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
        writeln!(writer, "  • Added default configuration to pyproject.toml.")?;
    } else {
        // 4. Neither exists: Create .cytoscnpy.toml
        let mut file = fs::File::create(&cytoscnpy_toml_path)?;
        writeln!(file, "{}", DEFAULT_CONFIG.trim())?;
        writeln!(
            writer,
            "  • Created .cytoscnpy.toml with default configuration."
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
                "  • .gitignore already contains {ignore_entry} - skipping."
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
