//! Security summaries retain errors that prevented a complete scan.

use cytoscnpy::analyzer::AnalysisResult;
use rmcp::model::{CallToolResult, Content};

pub(crate) fn render_summary(path: &str, result: &AnalysisResult) -> CallToolResult {
    let complete = result.parse_errors.is_empty();
    let recommendation = if !complete {
        "Security scan incomplete - resolve scan errors and run again"
    } else if result.secrets.is_empty() && result.danger.is_empty() {
        "✅ No security issues found"
    } else {
        "⚠️ Security issues detected - review and fix immediately"
    };
    let summary = serde_json::json!({
        "scan_type": "quick_security_scan",
        "path": path,
        "scan_complete": complete,
        "summary": {
            "secrets_found": result.secrets.len(),
            "dangerous_patterns": result.danger.len(),
            "total_issues": result.secrets.len() + result.danger.len(),
            "scan_errors": result.parse_errors.len(),
        },
        "secrets": result.secrets,
        "danger": result.danger,
        "parse_errors": result.parse_errors,
        "recommendation": recommendation,
    });
    let content = vec![Content::text(summary.to_string())];
    if complete {
        CallToolResult::success(content)
    } else {
        CallToolResult::error(content)
    }
}
