//! AST extraction and naming distribution scanning across Python files.

use rayon::prelude::*;
use std::path::{Path, PathBuf};

use super::classifier::{
    classify_identifier, is_anonymous_or_empty, is_dunder_name, is_unittest_fixture,
};
use super::types::{NamingDistributionResult, NamingOutlier, NamingStats, NamingStyle};
use crate::searchability::functions::{
    extract_definitions_from_file, extract_definitions_from_source, ClassDefinition,
    ExtractedDefinitions, FunctionDefinition,
};

/// Scans a collection of Python files and analyzes naming style distribution and consistency.
#[must_use]
pub fn scan_and_analyze_naming(files: &[PathBuf]) -> NamingDistributionResult {
    let all_defs: Vec<ExtractedDefinitions> = files
        .par_iter()
        .map(|file| extract_definitions_from_file(file))
        .collect();

    let mut functions = Vec::new();
    let mut classes = Vec::new();
    for def in all_defs {
        functions.extend(def.functions);
        classes.extend(def.classes);
    }

    analyze_naming(&functions, &classes)
}

/// Analyzes naming style distribution and outliers from a single Python source string.
#[must_use]
pub fn analyze_naming_from_source(source: &str, file: &Path) -> NamingDistributionResult {
    let defs = extract_definitions_from_source(source, file);
    analyze_naming(&defs.functions, &defs.classes)
}

/// Backward-compatible entry point that analyzes function definitions.
#[must_use]
pub fn analyze_naming_functions(functions: &[FunctionDefinition]) -> NamingDistributionResult {
    analyze_naming(functions, &[])
}

/// Computes naming statistics, dominant style, consistency ratio, and outliers.
#[must_use]
pub fn analyze_naming(
    functions: &[FunctionDefinition],
    classes: &[ClassDefinition],
) -> NamingDistributionResult {
    let mut stats = NamingStats::default();
    let mut classified_functions: Vec<(&FunctionDefinition, NamingStyle)> = Vec::new();
    let mut classified_classes: Vec<(&ClassDefinition, NamingStyle)> = Vec::new();

    for func in functions {
        let name = &func.name;
        if is_anonymous_or_empty(name) || is_unittest_fixture(name) {
            continue;
        }
        if is_dunder_name(name) {
            stats.dunder_count += 1;
            continue;
        }
        if let Some(style) = classify_identifier(name) {
            record_style(&mut stats, style);
            classified_functions.push((func, style));
        }
    }

    for class in classes {
        let name = &class.name;
        if is_anonymous_or_empty(name) {
            continue;
        }
        if is_dunder_name(name) {
            stats.dunder_count += 1;
            continue;
        }
        if let Some(style) = classify_identifier(name) {
            record_style(&mut stats, style);
            classified_classes.push((class, style));
        }
    }

    stats.total_identifiers = stats.classified_total();
    let dominant_func_style = if classified_functions.is_empty() {
        NamingStyle::SnakeCase
    } else {
        let mut func_stats = NamingStats::default();
        for (_, style) in &classified_functions {
            record_style(&mut func_stats, *style);
        }
        determine_dominant_style(&func_stats)
    };
    stats.dominant_style = dominant_func_style;

    let mut outliers = Vec::new();
    let mut conforming_count = 0usize;

    for (func, detected_style) in classified_functions {
        if detected_style == dominant_func_style {
            conforming_count += 1;
        } else {
            outliers.push(NamingOutlier {
                name: func.name.clone(),
                file: func.file.clone(),
                line: func.line,
                detected_style,
                expected_style: dominant_func_style,
            });
        }
    }

    for (class, detected_style) in classified_classes {
        if detected_style == NamingStyle::PascalCase {
            conforming_count += 1;
        } else {
            outliers.push(NamingOutlier {
                name: class.name.clone(),
                file: class.file.clone(),
                line: class.line,
                detected_style,
                expected_style: NamingStyle::PascalCase,
            });
        }
    }

    if stats.total_identifiers == 0 {
        stats.dominant_style_ratio = 1.0;
    } else {
        stats.dominant_style_ratio = conforming_count as f64 / stats.total_identifiers as f64;
    }

    outliers.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.name.cmp(&b.name))
    });

    NamingDistributionResult { stats, outliers }
}

fn record_style(stats: &mut NamingStats, style: NamingStyle) {
    match style {
        NamingStyle::SnakeCase => stats.snake_case_count += 1,
        NamingStyle::CamelCase => stats.camel_case_count += 1,
        NamingStyle::PascalCase => stats.pascal_case_count += 1,
        NamingStyle::ScreamingSnakeCase => stats.screaming_snake_count += 1,
        NamingStyle::Mixed => stats.mixed_count += 1,
    }
}

fn determine_dominant_style(stats: &NamingStats) -> NamingStyle {
    let styles = [
        (NamingStyle::SnakeCase, stats.snake_case_count),
        (NamingStyle::CamelCase, stats.camel_case_count),
        (NamingStyle::PascalCase, stats.pascal_case_count),
        (NamingStyle::ScreamingSnakeCase, stats.screaming_snake_count),
    ];

    let mut best_style = NamingStyle::SnakeCase;
    let mut best_count = 0;

    for (style, count) in styles {
        if count > best_count {
            best_count = count;
            best_style = style;
        }
    }

    best_style
}
