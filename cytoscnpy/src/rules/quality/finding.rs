use crate::rules::{Context, Finding, RuleMetadata};
use ruff_text_size::TextSize;
/// Create a Finding with accurate line/column mapping from a TextSize location.
pub(super) fn create_finding(
    msg: &str,
    metadata: RuleMetadata,
    context: &Context,
    location: TextSize,
    severity: &str,
) -> Finding {
    let line = context.line_index.line_index(location);
    let col = context.line_index.column_index(location);
    Finding {
        message: msg.to_owned(),
        rule_id: metadata.id.to_owned(),
        category: metadata.category.to_owned(),
        file: context.filename.clone(),
        line,
        col,
        severity: severity.to_owned(),
    }
}
