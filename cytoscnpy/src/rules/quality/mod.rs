use crate::config::Config;
use crate::rules::Rule;

mod best_practices;
mod complexity;
mod finding;
mod maintainability;
mod performance;

/// Category constants for quality rules.
pub const CAT_BEST_PRACTICES: &str = "Best Practices";
/// Category constants for maintainability rules.
pub const CAT_MAINTAINABILITY: &str = "Maintainability";
/// Category constants for performance rules.
pub const CAT_PERFORMANCE: &str = "Performance";

/// Returns a list of all quality rules based on configuration.
#[must_use]
pub fn get_quality_rules(config: &Config) -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(best_practices::MutableDefaultArgumentRule),
        Box::new(best_practices::BareExceptRule),
        Box::new(best_practices::DangerousComparisonRule),
        Box::new(maintainability::ArgumentCountRule::new(
            config.cytoscnpy.max_args.unwrap_or(5),
        )),
        Box::new(maintainability::FunctionLengthRule::new(
            config.cytoscnpy.max_lines.unwrap_or(50),
        )),
        Box::new(complexity::ComplexityRule::new(
            config.cytoscnpy.max_complexity.unwrap_or(10),
        )),
        Box::new(complexity::CognitiveComplexityRule::new(15)),
        Box::new(complexity::CohesionRule::new(1)),
        Box::new(maintainability::NestingRule::new(
            config.cytoscnpy.max_nesting.unwrap_or(3),
        )),
        // Performance Rules
        Box::new(performance::MembershipInListRule::new()),
        Box::new(performance::StringConcatInLoopRule::new()),
        Box::new(performance::UselessCastRule),
        Box::new(performance::RegexLoopRule::new()),
        Box::new(performance::AttributeChainHoistingRule::new()),
        Box::new(performance::PureCallHoistingRule::new()),
        Box::new(performance::ExceptionFlowInLoopRule::new()),
        Box::new(performance::IncorrectDictIteratorRule),
        Box::new(performance::GlobalUsageInLoopRule::new()),
        Box::new(performance::MemoryviewOverBytesRule::new()),
        Box::new(performance::UseTupleOverListRule::new()),
        Box::new(performance::ComprehensionSuggestionRule),
        Box::new(performance::FileReadMemoryRiskRule::new()),
        Box::new(performance::PandasChunksizeRiskRule),
    ]
}
