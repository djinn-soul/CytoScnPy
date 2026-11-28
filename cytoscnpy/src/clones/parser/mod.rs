//! Parser integration with `ruff_python_parser`.
//!
//! Extracts subtrees from Python source code for clone detection.

mod definitions;
mod enum_members;
mod expression_complex;
mod expressions;
mod extract;
mod interfaces;
mod patterns;
mod qualnames;
mod schema_fields;
mod statement_misc;
mod statements;
#[cfg(test)]
mod tests;
mod types;

pub use extract::extract_subtrees;
pub use types::{CloneFingerprint, Subtree, SubtreeNode, SubtreeType};

pub(crate) fn extract_subtrees_with_min_lines(
    source: &str,
    path: &std::path::PathBuf,
    min_lines: usize,
) -> Result<Vec<Subtree>, crate::clones::CloneError> {
    extract::extract_subtrees_with_min_lines(source, path, min_lines)
}

pub(crate) use extract::extract_subtrees_ast;

/// Returns true when `class_source` (a class definition sliced from a file)
/// derives from a recognised enum base. `module_source` supplies the imports
/// used to resolve qualified base names.
#[cfg(test)]
pub(crate) fn is_enum_class_source(class_source: &str, module_source: &str) -> bool {
    EnumClassDetector::new(module_source).is_enum_class(class_source)
}

/// Resolves whether classes in one module derive from an enum base. Parses
/// the module's imports once so many class checks share that work.
pub(crate) struct EnumClassDetector {
    imports: qualnames::ImportAliases,
}

impl EnumClassDetector {
    pub(crate) fn new(module_source: &str) -> Self {
        let imports = types::AstParser::parse(module_source)
            .map(|m| qualnames::ImportAliases::from_module(&m.body))
            .unwrap_or_default();
        Self { imports }
    }

    /// `class_source` is a class definition sliced from this module.
    pub(crate) fn is_enum_class(&self, class_source: &str) -> bool {
        let Ok(module) = types::AstParser::parse(class_source) else {
            return false;
        };
        let Some(ruff_python_ast::Stmt::ClassDef(class)) = module.body.first() else {
            return false;
        };
        enum_members::is_enum_class(class, &self.imports)
    }
}
