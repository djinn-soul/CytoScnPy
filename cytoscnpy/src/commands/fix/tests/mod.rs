use super::*;
use crate::analyzer::types::AnalysisResult;
use crate::commands::fix::apply::apply_dead_code_fix_to_file;
use crate::commands::fix::ranges::find_def_range;
use crate::visitor::Definition;
use smallvec::SmallVec;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

mod apply_basic_tests;
mod apply_edit_tests;
mod apply_json_tests;
mod import_tests;
mod ranges_tests;

fn create_definition(name: &str, def_type: &str, file: PathBuf, line: usize) -> Definition {
    Definition {
        name: name.to_owned(),
        full_name: name.to_owned(),
        simple_name: name.to_owned(),
        def_type: def_type.to_owned(),
        file: Arc::new(file),
        line,
        end_line: line + 1,
        col: 0,
        start_byte: 0,
        end_byte: 10,
        confidence: 100,
        references: 0,
        is_exported: false,
        in_init: false,
        is_framework_managed: false,
        base_classes: SmallVec::new(),
        is_type_checking: false,
        is_captured: false,
        cell_number: None,
        is_self_referential: false,
        message: None,
        fix: None,
        is_enum_member: false,
        is_constant: false,
        is_potential_secret: false,
        is_unreachable: false,
        category: crate::visitor::UnusedCategory::default(),
    }
}

fn create_definition_with_range(
    name: &str,
    def_type: &str,
    file: PathBuf,
    line: usize,
    start_byte: usize,
    end_byte: usize,
) -> Definition {
    Definition {
        start_byte,
        end_byte,
        ..create_definition(name, def_type, file, line)
    }
}
