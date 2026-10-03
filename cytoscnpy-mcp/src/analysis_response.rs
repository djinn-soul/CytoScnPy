//! Complete analysis responses retain findings while marking scan failures.

use cytoscnpy::analyzer::AnalysisResult;
use rmcp::model::{CallToolResult, Content};

pub(crate) fn render_analysis(result: &AnalysisResult) -> CallToolResult {
    let complete = result.parse_errors.is_empty();
    let mut payload = match serde_json::to_value(result) {
        Ok(payload) => payload,
        Err(error) => {
            return CallToolResult::error(vec![Content::text(format!(
                "Failed to serialize analysis: {error}"
            ))]);
        }
    };
    payload["scan_complete"] = serde_json::Value::Bool(complete);
    let content = vec![Content::text(payload.to_string())];
    if complete {
        CallToolResult::success(content)
    } else {
        CallToolResult::error(content)
    }
}
