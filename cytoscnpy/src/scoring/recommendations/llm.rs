//! Renders prioritized remediation recommendations into LLM-consumable markdown format.

use crate::scoring::types::ScoreResult;

/// Formats the score result and recommendations into an LLM-consumable remediation plan.
#[must_use]
pub fn format_llm_report(result: &ScoreResult) -> String {
    let mut out = String::new();

    out.push_str("# Codebase Remediation Plan (DeSlopify)\n\n");
    out.push_str("## Current Health Snapshot\n");
    out.push_str(&format!("- **Slop Index**: {} / 100\n", result.slop_index));
    out.push_str(&format!("- **Verdict Band**: {}\n", result.verdict));
    out.push_str(&format!("- **Raw Score**: {:.2} / 100\n", result.raw_score));
    out.push_str(&format!(
        "- **Active Context Size Multiplier**: {:.2}x\n",
        result.size_multiplier
    ));
    out.push_str(&format!(
        "- **Total Remediations**: {}\n\n",
        result.recommendations.len()
    ));

    if result.recommendations.is_empty() {
        let is_clean = result.slop_index == 0
            && (result.dimensions.is_empty() || result.dimensions.iter().all(|d| d.rating == 0));
        if is_clean {
            out.push_str("## Remediation Items\n\nNo active architectural friction or code slop detected. Codebase is in pristine condition.\n");
        } else {
            out.push_str("## Remediation Items\n\nNo automated remediation recommendations available for current findings.\n");
        }
        return out;
    }

    out.push_str("## Prioritized Action Items\n\n");
    out.push_str("Execute the following remediation steps in order of priority to achieve the greatest reduction in codebase friction:\n\n");

    for (idx, rec) in result.recommendations.iter().enumerate() {
        let step_num = idx + 1;
        out.push_str(&format!(
            "### [Step {step_num}] {} (-{} pts, Effort: {})\n\n",
            rec.title, rec.estimated_reduction, rec.effort
        ));
        out.push_str(&format!("- **Dimension**: {}\n", rec.dimension));
        out.push_str(&format!("- **Issue**: {}\n", rec.description));

        if !rec.affected_files.is_empty() {
            out.push_str("- **Target Files / Locations**:\n");
            for file in &rec.affected_files {
                out.push_str(&format!("  - `{file}`\n"));
            }
        }

        if !rec.action_steps.is_empty() {
            out.push_str("- **Implementation Steps**:\n");
            for step in &rec.action_steps {
                out.push_str(&format!("  1. {step}\n"));
            }
        }

        out.push('\n');
    }

    out.push_str("## Verification & Quality Gates\n\n");
    out.push_str(
        "After completing the action items, verify the changes and score reduction with:\n",
    );
    out.push_str("```bash\n");
    out.push_str("cytoscnpy score .\n");
    out.push_str("pytest\n");
    out.push_str("```\n");

    out
}
