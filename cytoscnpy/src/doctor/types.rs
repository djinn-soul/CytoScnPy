use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

/// Categories of development and repository configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ConfigCategory {
    /// Code formatting tools (e.g. Ruff format, Black, Prettier, rustfmt).
    Formatter,
    /// Code linters and static analyzers (e.g. Ruff, Flake8, Pylint, `ESLint`).
    Linter,
    /// Static type checkers (e.g. mypy, Pyright, `TypeScript`).
    TypeChecker,
    /// Test frameworks and test runners (e.g. pytest, unittest, Jest).
    TestFramework,
    /// Continuous integration configuration (e.g. `GitHub` Actions, `GitLab` CI).
    CI,
    /// Containerization and Docker configurations (e.g. Dockerfile, Docker Compose).
    Docker,
    /// Dependency specifications and package managers (e.g. pyproject.toml, package.json).
    DependencyManager,
    /// Locked dependency lockfiles (e.g. uv.lock, poetry.lock, Cargo.lock).
    Lockfile,
    /// Build scripts and task runners (e.g. Makefile, setup.py, Justfile).
    BuildScript,
    /// Repository documentation (e.g. README, CONTRIBUTING, ARCHITECTURE).
    Documentation,
    /// Git ignore files (.gitignore).
    GitIgnore,
    /// Editor and IDE settings (e.g. .editorconfig, .vscode).
    Editor,
}

impl fmt::Display for ConfigCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Formatter => write!(f, "Formatter"),
            Self::Linter => write!(f, "Linter"),
            Self::TypeChecker => write!(f, "Type Checker"),
            Self::TestFramework => write!(f, "Test Framework"),
            Self::CI => write!(f, "CI / CD"),
            Self::Docker => write!(f, "Docker / Container"),
            Self::DependencyManager => write!(f, "Dependencies"),
            Self::Lockfile => write!(f, "Lockfile"),
            Self::BuildScript => write!(f, "Build Script"),
            Self::Documentation => write!(f, "Documentation"),
            Self::GitIgnore => write!(f, "Git Ignore"),
            Self::Editor => write!(f, "Editor Config"),
        }
    }
}

/// A detected configuration file or directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectedConfig {
    /// Category of tooling.
    pub category: ConfigCategory,
    /// Tool or configuration name.
    pub name: String,
    /// Path to the configuration file or folder.
    pub path: PathBuf,
    /// Optional extra details extracted from the file.
    pub details: Option<String>,
}

/// Detailed inspection of tools configured in `pyproject.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PyProjectInspection {
    /// Whether Ruff is configured under `[tool.ruff]`.
    pub has_ruff: bool,
    /// Whether Black is configured under `[tool.black]`.
    pub has_black: bool,
    /// Whether mypy is configured under `[tool.mypy]`.
    pub has_mypy: bool,
    /// Whether Pyright is configured under `[tool.pyright]`.
    pub has_pyright: bool,
    /// Whether pytest is configured under `[tool.pytest.ini_options]`.
    pub has_pytest: bool,
    /// Whether Pylint is configured under `[tool.pylint]`.
    pub has_pylint: bool,
    /// Whether Poetry is configured under `[tool.poetry]`.
    pub has_poetry: bool,
    /// Whether Flit or Hatch is used as build backend.
    pub has_flit_or_hatch: bool,
    /// Declared build backend string.
    pub build_backend: Option<String>,
    /// Number of declared runtime dependencies.
    pub dependencies_count: usize,
    /// Number of declared development dependencies.
    pub dev_dependencies_count: usize,
}

/// Breakdown of code volume by programming language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageStats {
    /// Language name.
    pub language: String,
    /// Number of source files.
    pub file_count: usize,
    /// Number of lines of code.
    pub line_count: usize,
    /// Total bytes.
    pub byte_count: u64,
}

/// Structural repository metrics and test inventory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepoStructureStats {
    /// Total scanned files across all languages.
    pub total_files: usize,
    /// Total lines across all files.
    pub total_lines: usize,
    /// Total size in bytes.
    pub total_bytes: u64,
    /// Number of production source files.
    pub source_files: usize,
    /// Number of lines in source files.
    pub source_lines: usize,
    /// Number of test files.
    pub test_files: usize,
    /// Number of lines in test files.
    pub test_lines: usize,
    /// Ratio of test lines to source lines (e.g. 0.35 = 35%).
    pub test_to_source_ratio: f64,
    /// Average lines per file.
    pub avg_file_lines: usize,
    /// Path of the largest file.
    pub largest_file_path: String,
    /// Lines in the largest file.
    pub largest_file_lines: usize,
    /// Maximum directory depth in the project.
    pub max_directory_depth: usize,
    /// Language volume breakdown.
    pub languages: Vec<LanguageStats>,
}

/// Verdict on repository setup readiness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SetupVerdict {
    /// Complete setup with lockfile, CI, tests, and formatting.
    Ready,
    /// Solid setup missing only minor non-critical configurations.
    Solid,
    /// Incomplete setup missing key safeguards like tests or CI.
    Incomplete,
    /// High-friction setup missing lockfiles, tests, and linters.
    AtRisk,
}

impl fmt::Display for SetupVerdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ready => write!(f, "Ready"),
            Self::Solid => write!(f, "Solid"),
            Self::Incomplete => write!(f, "Incomplete"),
            Self::AtRisk => write!(f, "At Risk"),
        }
    }
}

/// Setup reliability score and checklist signals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupReliabilityScore {
    /// Overall score from 0 to 100.
    pub score: u32,
    /// Qualitative verdict.
    pub verdict: SetupVerdict,
    /// Whether a lockfile exists.
    pub has_lockfile: bool,
    /// Whether CI automation exists.
    pub has_ci: bool,
    /// Whether test framework is configured.
    pub has_tests: bool,
    /// Whether a linter is configured.
    pub has_linter: bool,
    /// Whether a code formatter is configured.
    pub has_formatter: bool,
    /// Whether a type checker is configured.
    pub has_type_checker: bool,
    /// Whether Docker or compose is configured.
    pub has_docker: bool,
    /// Whether README has setup / quickstart instructions.
    pub has_readme_setup: bool,
    /// Whether a build script or Makefile exists.
    pub has_build_script: bool,
    /// Whether a .gitignore is present.
    pub has_gitignore: bool,
    /// Actionable setup recommendations.
    pub recommendations: Vec<String>,
}

/// Configuration options for the doctor analysis.
#[derive(Debug, Clone, Default)]
pub struct DoctorConfig {
    /// Exit with error code 1 if critical setup items (tests, lockfile, linter) are missing.
    pub fail_on_missing: bool,
    /// Enable verbose logging output.
    pub verbose: bool,
    /// Excluded paths or folder patterns.
    pub excludes: Vec<String>,
}

/// Complete result of repository health and setup inspection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoctorResult {
    /// Root path analyzed.
    pub root_path: PathBuf,
    /// All detected configuration files.
    pub configs: Vec<DetectedConfig>,
    /// Deep inspection of pyproject.toml if present.
    pub pyproject: Option<PyProjectInspection>,
    /// Repository structure and language metrics.
    pub structure: RepoStructureStats,
    /// Setup reliability score and recommendations.
    pub reliability: SetupReliabilityScore,
}
