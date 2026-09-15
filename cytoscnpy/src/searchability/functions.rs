//! Function extraction and name collision detection via Python AST.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use ruff_python_ast::{self as ast, Stmt};
use ruff_python_parser::parse_module;
use ruff_text_size::Ranged;

use super::types::{FunctionCollisionEntry, FunctionLocation};
use crate::utils::LineIndex;

/// Structural and idiomatic method names that naturally repeat across many files.
const STRUCTURAL_NAMES: &[&str] = &[
    "setUp",
    "tearDown",
    "setUpClass",
    "tearDownClass",
    "setup_method",
    "teardown_method",
    "setup_class",
    "teardown_class",
    "beforeEach",
    "afterEach",
    "beforeAll",
    "afterAll",
    "run",
    "handle",
    "execute",
    "invoke",
    "apply",
    "main",
    "close",
    "reset",
    "start",
    "stop",
    "get",
    "set",
    "validate",
    "inner",
    "wrapper",
];

/// A raw function definition extracted from Python AST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDefinition {
    /// Function or method name.
    pub name: String,
    /// File containing the definition.
    pub file: PathBuf,
    /// 1-indexed source line.
    pub line: usize,
}

/// A raw class definition extracted from Python AST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassDefinition {
    /// Class name.
    pub name: String,
    /// File containing the definition.
    pub file: PathBuf,
    /// 1-indexed source line.
    pub line: usize,
}

/// Extracted functions and classes from Python source code.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExtractedDefinitions {
    /// Function and method definitions.
    pub functions: Vec<FunctionDefinition>,
    /// Class definitions.
    pub classes: Vec<ClassDefinition>,
}

/// Checks if a function name is structural/idiomatic and should be excluded from collision detection.
#[must_use]
pub fn is_structural_name(name: &str) -> bool {
    if name.is_empty() || name == "<anonymous>" {
        return true;
    }
    if name.starts_with("__") && name.ends_with("__") {
        return true;
    }
    STRUCTURAL_NAMES.contains(&name)
}

/// Extracts all function definitions from a Python source file.
#[must_use]
pub fn extract_functions_from_file(file: &Path) -> Vec<FunctionDefinition> {
    extract_definitions_from_file(file).functions
}

/// Extracts all function and class definitions from a Python source file.
#[must_use]
pub fn extract_definitions_from_file(file: &Path) -> ExtractedDefinitions {
    let Ok(content) = std::fs::read_to_string(file) else {
        return ExtractedDefinitions::default();
    };
    extract_definitions_from_source(&content, file)
}

/// Extracts function definitions from Python source code content.
#[must_use]
pub fn extract_functions_from_source(source: &str, file: &Path) -> Vec<FunctionDefinition> {
    extract_definitions_from_source(source, file).functions
}

/// Extracts function and class definitions from Python source code content.
#[must_use]
pub fn extract_definitions_from_source(source: &str, file: &Path) -> ExtractedDefinitions {
    let Ok(parsed) = parse_module(source) else {
        return ExtractedDefinitions::default();
    };

    let line_index = LineIndex::new(source);
    let mut defs = ExtractedDefinitions::default();
    traverse_stmts(
        &parsed.into_syntax().body,
        file,
        &line_index,
        &mut defs.functions,
        &mut defs.classes,
    );
    defs
}

fn traverse_stmts(
    stmts: &[Stmt],
    file: &Path,
    line_index: &LineIndex,
    functions: &mut Vec<FunctionDefinition>,
    classes: &mut Vec<ClassDefinition>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::FunctionDef(f) => {
                let line = line_index.line_index(f.range().start());
                functions.push(FunctionDefinition {
                    name: f.name.to_string(),
                    file: file.to_path_buf(),
                    line,
                });
                traverse_stmts(&f.body, file, line_index, functions, classes);
            }
            Stmt::ClassDef(c) => {
                let line = line_index.line_index(c.range().start());
                classes.push(ClassDefinition {
                    name: c.name.to_string(),
                    file: file.to_path_buf(),
                    line,
                });
                traverse_stmts(&c.body, file, line_index, functions, classes);
            }
            Stmt::If(i) => {
                traverse_stmts(&i.body, file, line_index, functions, classes);
                for clause in &i.elif_else_clauses {
                    traverse_stmts(&clause.body, file, line_index, functions, classes);
                }
            }
            Stmt::Try(t) => {
                traverse_stmts(&t.body, file, line_index, functions, classes);
                for h in &t.handlers {
                    let ast::ExceptHandler::ExceptHandler(handler) = h;
                    traverse_stmts(&handler.body, file, line_index, functions, classes);
                }
                traverse_stmts(&t.orelse, file, line_index, functions, classes);
                traverse_stmts(&t.finalbody, file, line_index, functions, classes);
            }
            Stmt::For(f) => {
                traverse_stmts(&f.body, file, line_index, functions, classes);
                traverse_stmts(&f.orelse, file, line_index, functions, classes);
            }
            Stmt::While(w) => {
                traverse_stmts(&w.body, file, line_index, functions, classes);
                traverse_stmts(&w.orelse, file, line_index, functions, classes);
            }
            Stmt::With(w) => {
                traverse_stmts(&w.body, file, line_index, functions, classes);
            }
            Stmt::Match(m) => {
                for case in &m.cases {
                    traverse_stmts(&case.body, file, line_index, functions, classes);
                }
            }
            _ => {}
        }
    }
}

/// Identifies function name collisions across distinct files (defined in 3 or more distinct files).
#[must_use]
pub fn find_function_collisions(
    functions: &[FunctionDefinition],
) -> (usize, Option<(String, usize)>, Vec<FunctionCollisionEntry>) {
    let mut name_to_locations: BTreeMap<String, Vec<FunctionLocation>> = BTreeMap::new();

    for func in functions {
        if is_structural_name(&func.name) {
            continue;
        }

        name_to_locations
            .entry(func.name.clone())
            .or_default()
            .push(FunctionLocation {
                file: func.file.clone(),
                line: func.line,
            });
    }

    let mut collisions = Vec::new();
    let mut worst: Option<(String, usize)> = None;

    for (name, mut locations) in name_to_locations {
        let distinct_files: BTreeSet<&PathBuf> = locations.iter().map(|l| &l.file).collect();
        let file_count = distinct_files.len();

        if file_count >= 3 {
            locations.sort();
            match &worst {
                Some((_, prev_count)) if file_count > *prev_count => {
                    worst = Some((name.clone(), file_count));
                }
                None => {
                    worst = Some((name.clone(), file_count));
                }
                _ => {}
            }
            collisions.push(FunctionCollisionEntry {
                function_name: name,
                file_count,
                locations,
            });
        }
    }

    collisions.sort_by(|a, b| {
        b.file_count
            .cmp(&a.file_count)
            .then_with(|| a.function_name.cmp(&b.function_name))
    });
    let collision_count = collisions.len();

    (collision_count, worst, collisions)
}
