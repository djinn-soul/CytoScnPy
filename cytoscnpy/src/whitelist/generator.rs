//! Whitelist generation from detected unused code.
//!
//! Generates Vulture-compatible Python whitelist files that can be used
//! to suppress false positives in subsequent scans.

use std::io::{self, Write};
use std::path::Path;

use crate::visitor::Definition;

/// Generate a Python whitelist file from detected dead code.
///
/// The output is valid Python syntax that can be parsed by CytoScnPy
/// or Vulture to suppress false positives.
///
/// # Arguments
/// * `definitions` - List of detected unused definitions.
/// * `output` - Writer for the output (typically stdout or a file).
///
/// # Example Output
///
/// ```python
/// # CytoScnPy Whitelist
/// # Generated from: src/
/// # Total entries: 3
/// #
/// # Each entry below represents a symbol that was detected as unused.
/// # Review each entry and remove any that are actually unused code.
/// # The remaining entries will be treated as "used" in future scans.
///
/// # src/api/handlers.py:15 - function
/// get_user_handler
///
/// # src/models/user.py:8 - class
/// User
///
/// # src/utils/helpers.py:42 - variable
/// helper_constant
/// ```
pub fn generate_whitelist(definitions: &[Definition], output: &mut dyn Write) -> io::Result<()> {
    // Header
    writeln!(output, "# CytoScnPy Whitelist")?;
    writeln!(output, "# Total entries: {}", definitions.len())?;
    writeln!(output, "#")?;
    writeln!(
        output,
        "# Each entry below represents a symbol that was detected as unused."
    )?;
    writeln!(
        output,
        "# Review each entry and remove any that are actually unused code."
    )?;
    writeln!(
        output,
        "# The remaining entries will be treated as \"used\" in future scans."
    )?;
    writeln!(output, "#")?;
    writeln!(output, "# Usage:")?;
    writeln!(output, "#   cytoscnpy src/ --make-whitelist > whitelist.py")?;
    writeln!(output, "#   cytoscnpy src/ --whitelist whitelist.py")?;
    writeln!(output)?;

    // Group by file for better organization
    let mut sorted_defs: Vec<_> = definitions.to_vec();
    sorted_defs.sort_by(|a, b| a.file.cmp(&b.file).then_with(|| a.line.cmp(&b.line)));

    let mut current_file: Option<&Path> = None;

    for item in &sorted_defs {
        // Add file separator when file changes
        if current_file != Some(item.file.as_ref()) {
            current_file = Some(item.file.as_ref());
            writeln!(output)?;
            writeln!(output, "# File: {}", item.file.display())?;
        }

        // Write the entry with comment
        writeln!(output, "# Line {} - {}", item.line, item.def_type)?;
        writeln!(output, "{}", item.name)?;
    }

    // Footer with instructions
    writeln!(output)?;
    writeln!(output, "# End of whitelist")?;
    writeln!(output, "# To use this whitelist:")?;
    writeln!(output, "#   1. Review each entry above")?;
    writeln!(
        output,
        "#   2. Remove entries for code that is truly unused"
    )?;
    writeln!(
        output,
        "#   3. Save this file as 'whitelist.py' in your project"
    )?;
    writeln!(
        output,
        "#   4. Run: cytoscnpy src/ --whitelist whitelist.py"
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use smallvec::smallvec;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn create_test_definition(name: &str, file: &str, line: usize, def_type: &str) -> Definition {
        Definition {
            name: name.to_string(),
            full_name: name.to_string(),
            simple_name: name.to_string(),
            def_type: def_type.to_string(),
            file: Arc::new(PathBuf::from(file)),
            line,
            end_line: line + 5,
            col: 0,
            start_byte: 0,
            end_byte: 100,
            confidence: 80,
            category: crate::visitor::UnusedCategory::default(),
            references: 0,
            is_exported: false,
            in_init: false,
            is_framework_managed: false,
            base_classes: smallvec![],
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
        }
    }

    #[test]
    fn test_generate_whitelist() {
        let definitions = vec![
            create_test_definition("unused_function", "src/api.py", 10, "function"),
            create_test_definition("UnusedClass", "src/models.py", 25, "class"),
            create_test_definition("helper_var", "src/api.py", 5, "variable"),
        ];
        let mut output = Vec::new();
        generate_whitelist(&definitions, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();

        // Check header
        assert!(output_str.contains("# CytoScnPy Whitelist"));
        assert!(output_str.contains("# Total entries: 3"));

        // Check entries
        assert!(output_str.contains("unused_function"));
        assert!(output_str.contains("UnusedClass"));
        assert!(output_str.contains("helper_var"));

        // Check file grouping
        assert!(output_str.contains("# File: src/api.py"));
        assert!(output_str.contains("# File: src/models.py"));
    }

    #[test]
    fn test_empty_whitelist() {
        let definitions: Vec<Definition> = vec![];
        let mut output = Vec::new();
        generate_whitelist(&definitions, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("# Total entries: 0"));
    }
}
