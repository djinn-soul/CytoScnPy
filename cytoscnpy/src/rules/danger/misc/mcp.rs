use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::{Expr, Stmt};
use ruff_text_size::Ranged;
use rustc_hash::FxHashSet;

use super::super::utils::{create_finding, get_call_name, is_literal_expr};

/// Rule for detecting `StdioServerParameters(command=<non-literal>)`.
///
/// Anthropic's MCP STDIO transport executes whatever string is passed as `command`
/// directly via the OS. If `command` is not a string literal (e.g. a variable,
/// f-string, or function call) an attacker who controls that value achieves RCE.
/// CVE-2025-6514, CVE-2025-53818.
pub struct McpStdioRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
    /// Local names that resolve to `StdioServerParameters` after import
    /// aliasing (e.g. `from mcp import StdioServerParameters as P`).
    aliases: FxHashSet<String>,
}

impl McpStdioRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self {
            metadata,
            aliases: FxHashSet::default(),
        }
    }

    fn is_stdio_params_call(&self, name: &str) -> bool {
        matches!(
            name,
            "StdioServerParameters"
                | "mcp.StdioServerParameters"
                | "mcp.client.stdio.StdioServerParameters"
                | "client.StdioServerParameters"
        ) || self.aliases.contains(name)
    }
}

impl Rule for McpStdioRule {
    fn name(&self) -> &'static str {
        "McpStdioRule"
    }

    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }

    fn enter_stmt(&mut self, stmt: &Stmt, _context: &Context) -> Option<Vec<Finding>> {
        // Record `from <anything> import StdioServerParameters as <alias>` so
        // calls through the local alias are still flagged. Any source module
        // is accepted because importing the symbol under a new name is the
        // signal that matters; restricting by module would silently miss
        // `from mcp.client.stdio import StdioServerParameters as P`.
        if let Stmt::ImportFrom(import) = stmt {
            for alias in &import.names {
                if alias.name.as_str() == "StdioServerParameters" {
                    if let Some(asname) = &alias.asname {
                        self.aliases.insert(asname.to_string());
                    }
                }
            }
        }
        None
    }

    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        let Expr::Call(call) = expr else {
            return None;
        };

        let name = get_call_name(&call.func)?;

        // Match StdioServerParameters (bare, qualified, or via import alias).
        if !self.is_stdio_params_call(name.as_str()) {
            return None;
        }

        // Check `command` keyword argument
        for kw in &call.arguments.keywords {
            if kw.arg.as_ref().is_some_and(|a| a == "command") && !is_literal_expr(&kw.value) {
                return Some(vec![create_finding(
                    "MCP StdioServerParameters: non-literal `command` enables arbitrary OS command execution (CVE-2025-6514).",
                    self.metadata,
                    context,
                    call.range().start(),
                    "CRITICAL",
                )]);
            }
        }

        // Also flag if `command` is the first positional arg and non-literal
        if let Some(first) = call.arguments.args.first() {
            if !is_literal_expr(first) {
                return Some(vec![create_finding(
                    "MCP StdioServerParameters: non-literal positional `command` enables arbitrary OS command execution (CVE-2025-6514).",
                    self.metadata,
                    context,
                    call.range().start(),
                    "CRITICAL",
                )]);
            }
        }

        None
    }
}
