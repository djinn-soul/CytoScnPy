use clap::Subcommand;

use super::{FilesArgs, MetricArgs, PathArgs, RankArgs};

#[derive(Subcommand, Debug)]
/// Available subcommands for specific metric calculations.
pub enum Commands {
    /// Calculate raw metrics (LOC, LLOC, SLOC, Comments, Multi, Blank)
    Raw {
        /// Common metric options (path, json, exclude, ignore, `output_file`).
        #[command(flatten)]
        common: MetricArgs,

        /// Show summary of gathered metrics.
        #[arg(long, short = 's')]
        summary: bool,
    },
    /// Calculate Cyclomatic Complexity
    Cc {
        /// Common metric options (path, json, exclude, ignore, `output_file`).
        #[command(flatten)]
        common: MetricArgs,

        /// Rank filtering options (min/max rank).
        #[command(flatten)]
        rank: RankArgs,

        /// Show average complexity.
        #[arg(long, short = 'a')]
        average: bool,

        /// Show total average complexity.
        #[arg(long)]
        total_average: bool,

        /// Show complexity score with rank.
        #[arg(long, short = 's')]
        show_complexity: bool,

        /// Ordering function (score, lines, alpha).
        #[arg(long, short = 'o')]
        order: Option<String>,

        /// Do not count assert statements.
        #[arg(long)]
        no_assert: bool,

        /// Output XML.
        #[arg(long)]
        xml: bool,

        /// Exit with code 1 if any block has complexity higher than this value.
        #[arg(long)]
        fail_threshold: Option<usize>,
    },
    /// Calculate Halstead Metrics
    Hal {
        /// Common metric options (path, json, exclude, ignore, `output_file`).
        #[command(flatten)]
        common: MetricArgs,

        /// Compute metrics on function level.
        #[arg(long, short = 'f')]
        functions: bool,
    },
    /// Calculate Maintainability Index
    Mi {
        /// Common metric options (path, json, exclude, ignore, `output_file`).
        #[command(flatten)]
        common: MetricArgs,

        /// Rank filtering options (min/max rank).
        #[command(flatten)]
        rank: RankArgs,

        /// Count multiline strings as comments (enabled by default).
        #[arg(long, short = 'm', default_value = "true", action = clap::ArgAction::Set)]
        multi: bool,

        /// Show actual MI value.
        #[arg(long, short = 's')]
        show: bool,

        /// Show average MI.
        #[arg(long, short = 'a')]
        average: bool,

        /// Exit with code 1 if any file has MI lower than this value.
        #[arg(long)]
        fail_threshold: Option<f64>,
    },
    /// Start MCP server for LLM integration (Claude Desktop, VS Code Copilot, etc.)
    #[command(name = "mcp-server")]
    McpServer,
    /// Initialize CytoScnPy configuration (pyproject.toml/.cytoscnpy.toml and .gitignore)
    Init,
    /// Generate project statistics report
    Stats {
        /// Path options (path vs root).
        #[command(flatten)]
        paths: PathArgs,

        /// Enable all analysis: secrets, danger, quality, and per-file metrics.
        #[arg(long, short = 'a')]
        all: bool,

        /// Scan for API keys/secrets.
        #[arg(long, short = 's')]
        secrets: bool,

        /// Scan for dangerous code patterns.
        #[arg(long, short = 'd')]
        danger: bool,

        /// Scan for code quality issues.
        #[arg(long, short = 'q')]
        quality: bool,

        /// Output JSON.
        #[arg(long)]
        json: bool,

        /// Output file path.
        #[arg(long, short = 'o')]
        output: Option<String>,

        /// Exclude folders.
        #[arg(long, alias = "exclude-folder")]
        exclude: Vec<String>,
    },
    /// Run the unified DeSlopify-compatible assessment
    Deslop {
        /// Unified analysis and CI options.
        #[command(flatten)]
        args: super::DeslopArgs,
    },
    /// Show per-file metrics table
    Files {
        /// Common options for listing files.
        #[command(flatten)]
        args: FilesArgs,
    },
    /// Analyze unused and missing dependencies
    Deps {
        /// Dependency analysis options.
        #[command(flatten)]
        args: super::DepsArgs,
    },
    /// Analyze codebase searchability, duplicate filenames, and function collisions
    Searchability {
        /// Searchability analysis options.
        #[command(flatten)]
        args: super::SearchabilityArgs,
    },
    /// Analyze identifier naming-style distribution, consistency, and outliers
    Naming {
        /// Naming analysis options.
        #[command(flatten)]
        args: super::NamingArgs,
    },
    /// Detect TODO/FIXME/HACK/XXX annotations, debug prints, and commented-out code
    Todos {
        /// Todos analysis options.
        #[command(flatten)]
        args: super::TodosArgs,
    },
    /// Detect mutable global state across Python and polyglot source files
    #[command(alias = "mutable-globals")]
    Globals {
        /// Globals analysis options.
        #[command(flatten)]
        args: super::GlobalsArgs,
    },
    /// Detect bare-except and empty exception-handler anti-patterns
    #[command(alias = "bare-except")]
    Exceptions {
        /// Exceptions analysis options.
        #[command(flatten)]
        args: super::ExceptionsArgs,
    },
    /// Detect wildcard imports (`from module import *`) across Python source files
    #[command(alias = "star-imports")]
    Wildcards {
        /// Wildcards analysis options.
        #[command(flatten)]
        args: super::WildcardsArgs,
    },
    /// Detect module-level import-time side effects
    #[command(alias = "side-effects")]
    SideEffects {
        /// Side effects analysis options.
        #[command(flatten)]
        args: super::SideEffectsArgs,
    },
    /// Detect singleton patterns in Python classes
    #[command(alias = "singleton")]
    Singletons {
        /// Singletons analysis options.
        #[command(flatten)]
        args: super::SingletonsArgs,
    },
    /// Detect anti-patterns (magic numbers and deeply nested callbacks) in Python code
    #[command(
        name = "anti-patterns",
        alias = "antipatterns",
        alias = "magic-numbers",
        alias = "callbacks"
    )]
    AntiPatterns {
        /// Anti-patterns analysis options.
        #[command(flatten)]
        args: super::AntiPatternsArgs,
    },
    /// Analyze duplicate-code clusters and non-overlapping duplicate-line totals
    #[command(alias = "dupes", alias = "clones-summary")]
    Duplicates {
        /// Duplicates analysis options.
        #[command(flatten)]
        args: super::DuplicatesArgs,
    },
    /// Analyze Python module architecture, import graph, and circular dependencies
    Graph {
        /// Graph analysis options.
        #[command(flatten)]
        args: super::GraphArgs,
    },
    /// Analyze potentially unreferenced large functions in files with no incoming imports
    #[command(alias = "dead-functions", alias = "isolated-functions")]
    Unreferenced {
        /// Unreferenced functions analysis options.
        #[command(flatten)]
        args: super::UnreferencedArgs,
    },
    /// Analyze Git-aware code churn, hotspots, and LLM token budget
    Context {
        /// Context analysis options.
        #[command(flatten)]
        args: super::ContextArgs,
    },
    /// Calculate weighted Slop Index (0-100), verdict bands, and dimension breakdown
    #[command(alias = "slop-index", alias = "slop")]
    Score {
        /// Scoring options.
        #[command(flatten)]
        args: super::ScoreArgs,
    },
    /// Check repository health, setup reliability, and configuration metadata
    #[command(alias = "health")]
    Doctor {
        /// Doctor analysis options.
        #[command(flatten)]
        args: super::DoctorArgs,
    },
}
