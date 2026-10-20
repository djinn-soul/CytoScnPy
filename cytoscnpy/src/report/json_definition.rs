//! Key-sorted JSON view of a [`Definition`].
//!
//! Dead-code findings dominate large payloads. Serializing them through this
//! borrowed view yields the bytes of a sorted `serde_json::Value` without
//! building one per definition.

use std::path::PathBuf;

use serde::Serialize;

use crate::analyzer::types::FixSuggestion;
use crate::visitor::{Definition, DefinitionType, UnusedCategory};

/// Fields are declared in key order and mirror the serde attributes on
/// [`Definition`].
#[derive(Serialize)]
pub(super) struct SortedDefinition<'a> {
    base_classes: &'a [String],
    category: &'a UnusedCategory,
    #[serde(skip_serializing_if = "Option::is_none")]
    cell_number: Option<usize>,
    col: usize,
    confidence: u8,
    def_type: DefinitionType,
    end_byte: usize,
    end_line: usize,
    file: &'a PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    fix: Option<SortedFix<'a>>,
    full_name: &'a str,
    in_init: bool,
    is_captured: bool,
    is_constant: bool,
    is_enum_member: bool,
    is_exported: bool,
    is_framework_managed: bool,
    is_potential_secret: bool,
    is_self_referential: bool,
    is_type_checking: bool,
    is_unreachable: bool,
    line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<&'a str>,
    name: &'a str,
    references: usize,
    simple_name: &'a str,
    start_byte: usize,
}

#[derive(Serialize)]
struct SortedFix<'a> {
    end_byte: usize,
    replacement: &'a str,
    start_byte: usize,
}

impl<'a> From<&'a Definition> for SortedDefinition<'a> {
    fn from(definition: &'a Definition) -> Self {
        // Exhaustive, so a new `Definition` field fails to compile here
        // until the view serializes it.
        let Definition {
            name,
            full_name,
            simple_name,
            def_type,
            file,
            line,
            end_line,
            col,
            start_byte,
            end_byte,
            confidence,
            category,
            references,
            is_exported,
            in_init,
            is_framework_managed,
            base_classes,
            is_type_checking,
            is_captured,
            cell_number,
            is_self_referential,
            message,
            fix,
            is_enum_member,
            is_constant,
            is_potential_secret,
            is_unreachable,
        } = definition;
        Self {
            base_classes: base_classes.as_slice(),
            category,
            cell_number: *cell_number,
            col: *col,
            confidence: *confidence,
            def_type: *def_type,
            end_byte: *end_byte,
            end_line: *end_line,
            file,
            fix: fix.as_deref().map(|fix: &FixSuggestion| SortedFix {
                end_byte: fix.end_byte,
                replacement: &fix.replacement,
                start_byte: fix.start_byte,
            }),
            full_name,
            in_init: *in_init,
            is_captured: *is_captured,
            is_constant: *is_constant,
            is_enum_member: *is_enum_member,
            is_exported: *is_exported,
            is_framework_managed: *is_framework_managed,
            is_potential_secret: *is_potential_secret,
            is_self_referential: *is_self_referential,
            is_type_checking: *is_type_checking,
            is_unreachable: *is_unreachable,
            line: *line,
            message: message.as_deref(),
            name,
            references: *references,
            simple_name,
            start_byte: *start_byte,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SortedDefinition;
    use crate::analyzer::types::FixSuggestion;
    use crate::visitor::{Definition, DefinitionType, UnusedCategory};

    fn assert_matches_value(definition: &Definition) {
        let view = SortedDefinition::from(definition);
        let value = serde_json::to_value(definition).unwrap();
        assert_eq!(serde_json::to_value(&view).unwrap(), value);
        // Declaration order must match the sorted keys of a `Value`.
        assert_eq!(
            serde_json::to_string_pretty(&view).unwrap(),
            serde_json::to_string_pretty(&value).unwrap()
        );
    }

    #[test]
    fn view_serializes_like_a_sorted_value() {
        assert_matches_value(&Definition::default());
        assert_matches_value(&Definition {
            name: "pkg.mod.Widget".to_owned(),
            full_name: "pkg.mod.Widget".to_owned(),
            simple_name: "Widget".to_owned(),
            def_type: DefinitionType::Class,
            file: std::sync::Arc::new("pkg/mod.py".into()),
            line: 3,
            end_line: 9,
            col: 1,
            start_byte: 10,
            end_byte: 90,
            confidence: 80,
            category: UnusedCategory::PossiblyIntentional,
            references: 2,
            is_exported: true,
            in_init: true,
            is_framework_managed: true,
            base_classes: ["Base".to_owned(), "Mixin \"quoted\"".to_owned()]
                .into_iter()
                .collect(),
            is_type_checking: true,
            is_captured: true,
            cell_number: Some(4),
            is_self_referential: true,
            message: Some("multi\nline".to_owned()),
            fix: Some(Box::new(FixSuggestion {
                start_byte: 10,
                end_byte: 90,
                replacement: "pass\n".to_owned(),
            })),
            is_enum_member: true,
            is_constant: true,
            is_potential_secret: true,
            is_unreachable: true,
        });
    }
}
