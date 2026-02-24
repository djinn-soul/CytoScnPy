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
    /// Generate comprehensive project statistics report
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
    /// Show per-file metrics table
    Files {
        /// Common options for listing files.
        #[command(flatten)]
        args: FilesArgs,
    },
}
