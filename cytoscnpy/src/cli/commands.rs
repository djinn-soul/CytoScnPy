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
    /// Analyze Python module architecture, import graph, and circular dependencies
    Graph {
        /// Path options (paths vs root).
        #[command(flatten)]
        paths: PathArgs,
        /// Output JSON format.
        #[arg(long)]
        json: bool,
        /// Only show circular dependency cycles.
        #[arg(long)]
        cycles_only: bool,
        /// Exit with code 1 if any circular dependencies are detected.
        #[arg(long)]
        fail_on_cycles: bool,
        /// Exit with code 1 if any god modules are detected.
        #[arg(long)]
        fail_on_god_modules: bool,
        /// Exclude folders.
        #[arg(long, alias = "exclude-folder")]
        exclude: Vec<String>,
        /// Output file path.
        #[arg(long, short = 'o')]
        output_file: Option<String>,
    },
    /// Analyze Git-aware code churn, hotspots, and LLM token budget
    Context {
        /// Path options (paths vs root).
        #[command(flatten)]
        paths: PathArgs,
        /// Lookback window in months for Git activity (default: auto-scaled to repo age).
        #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
        git_months: Option<u32>,
        /// Usable LLM context capacity in tokens (default: 176000).
        #[arg(long)]
        context_budget: Option<usize>,
        /// Disable Git history analysis (pure static context estimation).
        #[arg(long)]
        no_git: bool,
        /// Output JSON format.
        #[arg(long)]
        json: bool,
        /// Only show churn-complexity hotspot files.
        #[arg(long)]
        hotspots_only: bool,
        /// Exit with code 1 if severe (Critical or High) hotspots are detected.
        #[arg(long)]
        fail_on_hotspots: bool,
        /// Exclude folders.
        #[arg(long, alias = "exclude-folder")]
        exclude: Vec<String>,
        /// Output file path.
        #[arg(long, short = 'o')]
        output_file: Option<String>,
    },
    /// Check repository health, setup reliability, and configuration metadata
    #[command(alias = "health")]
    Doctor {
        /// Path options (paths vs root).
        #[command(flatten)]
        paths: PathArgs,
        /// Output JSON format.
        #[arg(long)]
        json: bool,
        /// Exit with code 1 if critical or recommended setup items are missing or failing.
        #[arg(long)]
        fail_on_missing: bool,
        /// Exclude folders.
        #[arg(long, alias = "exclude-folder")]
        exclude: Vec<String>,
        /// Output file path.
        #[arg(long, short = 'o')]
        output_file: Option<String>,
    },
}
