//! The kinds of definitions the visitor tracks and their textual forms.

use super::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
/// Kinds of definitions tracked by the analyzer.
pub enum DefinitionType {
    /// Top-level or nested function definition.
    #[default]
    Function,
    /// Method definition inside a class.
    Method,
    /// Class definition.
    Class,
    /// Import binding definition.
    Import,
    /// Variable assignment definition.
    Variable,
    /// Function parameter definition.
    Parameter,
}

impl DefinitionType {
    /// Returns the stable lowercase label used in JSON and reports.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::Class => "class",
            Self::Import => "import",
            Self::Variable => "variable",
            Self::Parameter => "parameter",
        }
    }

    /// Returns the rule ID reported when a definition of this kind is
    /// unused, which `# noqa: unused-import` and the like suppress.
    #[must_use]
    pub const fn unused_rule_id(self) -> &'static str {
        match self {
            Self::Function => "unused-function",
            Self::Method => "unused-method",
            Self::Class => "unused-class",
            Self::Import => "unused-import",
            Self::Variable => "unused-variable",
            Self::Parameter => "unused-parameter",
        }
    }

    /// Every kind, in declaration order.
    pub const ALL: [Self; 6] = [
        Self::Function,
        Self::Method,
        Self::Class,
        Self::Import,
        Self::Variable,
        Self::Parameter,
    ];

    /// Returns every rule ID whose `# noqa` suppresses an unused definition of
    /// this kind: its own ID, [`DEAD_CODE_RULE_ID`], and the pyflakes/ruff code
    /// existing comments commonly use. The JSON report publishes this so the
    /// editor extension applies the same suppressions.
    #[must_use]
    pub const fn suppression_codes(self) -> &'static [&'static str] {
        match self {
            Self::Function => &["unused-function", DEAD_CODE_RULE_ID],
            Self::Method => &["unused-method", DEAD_CODE_RULE_ID],
            Self::Class => &["unused-class", DEAD_CODE_RULE_ID],
            Self::Import => &["unused-import", DEAD_CODE_RULE_ID, "F401"],
            Self::Variable => &["unused-variable", DEAD_CODE_RULE_ID, "F841"],
            Self::Parameter => &["unused-parameter", DEAD_CODE_RULE_ID],
        }
    }
}

/// Rule ID that suppresses every kind of unused definition.
pub const DEAD_CODE_RULE_ID: &str = "CSP-V001";

impl fmt::Display for DefinitionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl PartialEq<&str> for DefinitionType {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<DefinitionType> for &str {
    fn eq(&self, other: &DefinitionType) -> bool {
        *self == other.as_str()
    }
}

impl TryFrom<&str> for DefinitionType {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "function" => Ok(Self::Function),
            "method" => Ok(Self::Method),
            "class" => Ok(Self::Class),
            "import" => Ok(Self::Import),
            "variable" => Ok(Self::Variable),
            "parameter" => Ok(Self::Parameter),
            _ => Err(format!("unknown definition type: {value}")),
        }
    }
}
