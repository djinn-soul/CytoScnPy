//! Dead code fix command.

use crate::fix::{ByteRangeRewriter, Edit};

use anyhow::Result;
use colored::Colorize;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Options for dead code fix
#[derive(Debug, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct DeadCodeFixOptions {
    /// Minimum confidence threshold for auto-fix (0-100)
    pub min_confidence: u8,
    /// Dry-run mode (show what would change)
    pub dry_run: bool,
    /// Fix functions
    pub fix_functions: bool,
    /// Fix classes
    pub fix_classes: bool,
    /// Fix imports
    pub fix_imports: bool,
    /// Verbose output
    pub verbose: bool,
    /// Use CST for precise fixing
    pub with_cst: bool,
    /// Analysis root for path containment
    pub analysis_root: PathBuf,
}

/// Result of dead code fix operation
#[derive(Debug, Serialize)]
pub struct FixResult {
    /// File that was fixed
    pub file: String,
    /// Number of items removed
    pub items_removed: usize,
    /// Names of removed items
    pub removed_names: Vec<String>,
}

/// Apply --fix to dead code findings.
///
/// # Errors
///
/// Returns an error if file I/O fails or fix fails.
#[allow(clippy::too_many_lines)]
pub fn run_fix_deadcode<W: Write>(
    results: &crate::analyzer::AnalysisResult,
    options: &DeadCodeFixOptions,
    mut writer: W,
) -> Result<Vec<FixResult>> {
    if options.dry_run {
        writeln!(
            writer,
            "\n{}",
            "[DRY-RUN] Dead code that would be removed:".yellow()
        )?;
    } else {
        writeln!(writer, "\n{}", "Applying dead code fixes...".cyan())?;
    }

    let items_by_file = collect_items_to_fix(results, options);

    if items_by_file.is_empty() {
        writeln!(
            writer,
            "  No items with confidence >= {} to fix.",
            options.min_confidence
        )?;
        return Ok(vec![]);
    }

    print_fix_stats(&mut writer, &items_by_file, results, options)?;

    let mut all_results = Vec::new();

    for (file_path, items) in items_by_file {
        if let Some(res) = apply_dead_code_fix_to_file(&mut writer, &file_path, &items, options)? {
            all_results.push(res);
        }
    }

    Ok(all_results)
}

fn find_def_range(
    body: &[ruff_python_ast::Stmt],
    name: &str,
    def_type: &str,
) -> Option<(usize, usize)> {
    use ruff_python_ast::Stmt;
    use ruff_text_size::Ranged;

    for stmt in body {
        match stmt {
            Stmt::FunctionDef(f) if def_type == "function" => {
                if f.name.as_str() == name {
                    return Some((f.range().start().to_usize(), f.range().end().to_usize()));
                }
            }
            Stmt::ClassDef(c) if def_type == "class" => {
                if c.name.as_str() == name {
                    return Some((c.range().start().to_usize(), c.range().end().to_usize()));
                }
            }
            Stmt::Import(i) if def_type == "import" => {
                for alias in &i.names {
                    let import_name = alias.asname.as_ref().unwrap_or(&alias.name);
                    if import_name.as_str() == name {
                        return Some((i.range().start().to_usize(), i.range().end().to_usize()));
                    }
                }
            }
            Stmt::ImportFrom(i) if def_type == "import" => {
                for alias in &i.names {
                    let import_name = alias.asname.as_ref().unwrap_or(&alias.name);
                    if import_name.as_str() == name && i.names.len() == 1 {
                        return Some((i.range().start().to_usize(), i.range().end().to_usize()));
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn collect_items_to_fix<'a>(
    results: &'a crate::analyzer::AnalysisResult,
    options: &DeadCodeFixOptions,
) -> HashMap<PathBuf, Vec<(&'static str, &'a crate::visitor::Definition)>> {
    let mut items_by_file: HashMap<PathBuf, Vec<(&'static str, &crate::visitor::Definition)>> =
        HashMap::new();

    if options.fix_functions {
        for def in &results.unused_functions {
            if def.confidence >= options.min_confidence {
                items_by_file
                    .entry((*def.file).clone())
                    .or_default()
                    .push(("function", def));
            }
        }
    }

    if options.fix_classes {
        for def in &results.unused_classes {
            if def.confidence >= options.min_confidence {
                items_by_file
                    .entry((*def.file).clone())
                    .or_default()
                    .push(("class", def));
            }
        }
    }

    if options.fix_imports {
        for def in &results.unused_imports {
            if def.confidence >= options.min_confidence {
                items_by_file
                    .entry((*def.file).clone())
                    .or_default()
                    .push(("import", def));
            }
        }
    }

    items_by_file
}

fn print_fix_stats<W: Write>(
    writer: &mut W,
    items_by_file: &HashMap<PathBuf, Vec<(&'static str, &crate::visitor::Definition)>>,
    results: &crate::analyzer::AnalysisResult,
    options: &DeadCodeFixOptions,
) -> Result<()> {
    if options.verbose {
        let total_items: usize = items_by_file.values().map(Vec::len).sum();
        let files_count = items_by_file.len();

        let mut func_count = 0;
        let mut class_count = 0;
        let mut import_count = 0;
        for items in items_by_file.values() {
            for (item_type, _) in items {
                match *item_type {
                    "function" => func_count += 1,
                    "class" => class_count += 1,
                    "import" => import_count += 1,
                    _ => {}
                }
            }
        }

        writeln!(writer, "[VERBOSE] Fix Statistics:")?;
        writeln!(writer, "   Files to modify: {files_count}")?;
        writeln!(writer, "   Items to remove: {total_items}")?;
        writeln!(writer, "   Functions: {func_count}")?;
        writeln!(writer, "   Classes: {class_count}")?;
        writeln!(writer, "   Imports: {import_count}")?;

        let skipped_funcs = results
            .unused_functions
            .iter()
            .filter(|d| d.confidence < options.min_confidence)
            .count();
        let skipped_classes = results
            .unused_classes
            .iter()
            .filter(|d| d.confidence < options.min_confidence)
            .count();
        let skipped_imports = results
            .unused_imports
            .iter()
            .filter(|d| d.confidence < options.min_confidence)
            .count();
        let total_skipped = skipped_funcs + skipped_classes + skipped_imports;

        if total_skipped > 0 {
            writeln!(
                writer,
                "   Skipped (confidence < {}%): {}",
                options.min_confidence, total_skipped
            )?;
        }
        writeln!(writer)?;
    }
    Ok(())
}

fn apply_dead_code_fix_to_file<W: Write>(
    writer: &mut W,
    file_path: &Path,
    items: &[(&'static str, &crate::visitor::Definition)],
    options: &DeadCodeFixOptions,
) -> Result<Option<FixResult>> {
    #[cfg(feature = "cst")]
    use crate::cst::{AstCstMapper, CstParser};

    let file_path = crate::utils::validate_output_path(file_path, Some(&options.analysis_root))?;

    let content = match fs::read_to_string(&file_path) {
        Ok(c) => c,
        Err(e) => {
            writeln!(
                writer,
                "  {} {}: {}",
                "Skip:".yellow(),
                crate::utils::normalize_display_path(&file_path),
                e
            )?;
            return Ok(None);
        }
    };

    let parsed = match ruff_python_parser::parse_module(&content) {
        Ok(p) => p,
        Err(e) => {
            writeln!(
                writer,
                "  {} {}: {}",
                "Parse error:".red(),
                crate::utils::normalize_display_path(&file_path),
                e
            )?;
            return Ok(None);
        }
    };

    let module = parsed.into_syntax();
    let mut edits = Vec::new();
    let mut removed_names = Vec::new();

    #[cfg(feature = "cst")]
    let cst_mapper = if options.with_cst {
        CstParser::new()
            .ok()
            .and_then(|mut p| p.parse(&content).ok())
            .map(AstCstMapper::new)
    } else {
        None
    };

    for (item_type, def) in items {
        if let Some((start, end)) = find_def_range(&module.body, &def.simple_name, item_type) {
            let start_byte = start;
            let end_byte = end;

            #[cfg(feature = "cst")]
            let (start_byte, end_byte) = if let Some(mapper) = &cst_mapper {
                mapper.precise_range_for_def(start, end)
            } else {
                (start_byte, end_byte)
            };

            if options.dry_run {
                writeln!(
                    writer,
                    "  Would remove {} '{}' at {}:{}",
                    item_type,
                    def.simple_name,
                    crate::utils::normalize_display_path(&file_path),
                    def.line
                )?;
            } else {
                edits.push(Edit::delete(start_byte, end_byte));
                removed_names.push(def.simple_name.clone());
            }
        }
    }

    if !options.dry_run && !edits.is_empty() {
        let mut rewriter = ByteRangeRewriter::new(content);
        rewriter.add_edits(edits);
        if let Ok(fixed) = rewriter.apply() {
            let count = removed_names.len();
            fs::write(&file_path, fixed)?;
            writeln!(
                writer,
                "  {} {} ({} removed)",
                "Fixed:".green(),
                crate::utils::normalize_display_path(&file_path),
                count
            )?;
            return Ok(Some(FixResult {
                file: file_path.to_string_lossy().to_string(),
                items_removed: count,
                removed_names,
            }));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::types::{AnalysisResult, AnalysisSummary};
    use crate::visitor::Definition;
    use smallvec::SmallVec;
    use std::sync::Arc;
    use tempfile::TempDir;

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

    fn create_empty_analysis_result() -> AnalysisResult {
        AnalysisResult {
            unused_functions: Vec::new(),
            unused_methods: Vec::new(),
            unused_imports: Vec::new(),
            unused_classes: Vec::new(),
            unused_variables: Vec::new(),
            unused_parameters: Vec::new(),
            secrets: Vec::new(),
            danger: Vec::new(),
            quality: Vec::new(),
            taint_findings: Vec::new(),
            parse_errors: Vec::new(),
            clones: Vec::new(),
            file_metrics: Vec::new(),
            analysis_summary: AnalysisSummary {
                total_files: 0,
                secrets_count: 0,
                danger_count: 0,
                quality_count: 0,
                taint_count: 0,
                parse_errors_count: 0,
                total_lines_analyzed: 0,
                total_definitions: 0,
                average_complexity: 0.0,
                average_mi: 0.0,
                total_directories: 0,
                total_size: 0.0,
                functions_count: 0,
                classes_count: 0,
                raw_metrics: crate::raw_metrics::RawMetrics::default(),
                halstead_metrics: crate::halstead::HalsteadMetrics::default(),
            },
        }
    }

    #[test]
    fn test_find_def_range_function() {
        let source = "
def used(): pass

def unused():
    pass
";
        let parsed = ruff_python_parser::parse_module(source).unwrap();
        let body = parsed.into_syntax().body;

        let range = find_def_range(&body, "unused", "function");
        assert!(range.is_some());
        let (start, _end) = range.unwrap();
        assert!(start > 15);
    }

    #[test]
    fn test_find_def_range_class() {
        let source = "
class Used: pass

class Unused:
    pass
";
        let parsed = ruff_python_parser::parse_module(source).unwrap();
        let body = parsed.into_syntax().body;

        let range = find_def_range(&body, "Unused", "class");
        assert!(range.is_some());
    }

    #[test]
    fn test_find_def_range_import() {
        let source = "
import used
import unused
";
        let parsed = ruff_python_parser::parse_module(source).unwrap();
        let body = parsed.into_syntax().body;

        let range = find_def_range(&body, "unused", "import");
        assert!(range.is_some());
    }

    #[test]
    fn test_run_fix_deadcode_dry_run() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.py");
        let source = "
def unused_function():
    pass
";
        std::fs::write(&file_path, source).unwrap();

        let def = create_definition("unused_function", "function", file_path.clone(), 2);

        let mut results = create_empty_analysis_result();
        results.unused_functions.push(def);

        let options = DeadCodeFixOptions {
            min_confidence: 60,
            dry_run: true,
            fix_functions: true,
            fix_classes: false,
            fix_imports: false,
            verbose: true,
            with_cst: false,
            analysis_root: dir.path().to_path_buf(),
        };

        let mut buffer = Vec::new();
        let fix_results = run_fix_deadcode(&results, &options, &mut buffer).unwrap();

        assert!(fix_results.is_empty());

        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("[DRY-RUN]"));
        assert!(output.contains("Would remove function 'unused_function'"));

        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, source);
    }

    #[test]
    fn test_run_fix_deadcode_apply() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.py");
        let source = "
def unused_function():
    pass
";
        std::fs::write(&file_path, source).unwrap();

        let def = create_definition("unused_function", "function", file_path.clone(), 2);

        let mut results = create_empty_analysis_result();
        results.unused_functions.push(def);

        let options = DeadCodeFixOptions {
            min_confidence: 60,
            dry_run: false,
            fix_functions: true,
            fix_classes: false,
            fix_imports: false,
            verbose: false,
            with_cst: false,
            analysis_root: dir.path().to_path_buf(),
        };

        let mut buffer = Vec::new();
        let fix_results = run_fix_deadcode(&results, &options, &mut buffer).unwrap();

        assert_eq!(fix_results.len(), 1);
        assert_eq!(fix_results[0].items_removed, 1);

        let content = std::fs::read_to_string(&file_path).unwrap();
        assert!(content.trim().is_empty());
    }

    #[test]
    fn test_collect_items_to_fix() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.py");
        let def = create_definition("test", "function", file_path.clone(), 1);

        let mut results = create_empty_analysis_result();
        results.unused_functions.push(def);
        results
            .unused_classes
            .push(create_definition("Class", "class", file_path.clone(), 10));
        results
            .unused_imports
            .push(create_definition("imp", "import", file_path, 20));

        let options = DeadCodeFixOptions {
            min_confidence: 60,
            fix_functions: true,
            fix_classes: true,
            fix_imports: true,
            ..DeadCodeFixOptions::default()
        };

        let collected = collect_items_to_fix(&results, &options);
        assert_eq!(collected.values().next().unwrap().len(), 3);
    }

    #[test]
    fn test_find_def_range_import_from_multi() {
        let source = "from mod import a, b, c";
        let parsed = ruff_python_parser::parse_module(source).unwrap();
        let body = parsed.into_syntax().body;

        let range = find_def_range(&body, "a", "import");
        assert!(range.is_none());
    }

    #[test]
    fn test_apply_fix_parse_error() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.py");
        std::fs::write(&file_path, "invalid python code (((( (").unwrap();

        let def = create_definition("f", "function", file_path.clone(), 1);
        let options = DeadCodeFixOptions {
            analysis_root: dir.path().to_path_buf(),
            ..DeadCodeFixOptions::default()
        };

        let mut buffer = Vec::new();
        let res =
            apply_dead_code_fix_to_file(&mut buffer, &file_path, &[("function", &def)], &options)
                .unwrap();
        assert!(res.is_none());
        assert!(String::from_utf8(buffer).unwrap().contains("Parse error:"));
    }
}
