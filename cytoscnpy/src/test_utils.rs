use crate::utils::LineIndex;
use ruff_python_ast::{Expr, Stmt};
use std::path::Path;

use crate::constants::{TEST_DECOR_RE, TEST_METHOD_PATTERN};

/// A visitor that detects test-related code.
///
/// This is important because "unused" code in test files (like helper functions or fixtures)
/// is often valid and shouldn't be reported as dead code.
pub struct TestAwareVisitor<'a> {
    /// Indicates if the file being visited is considered a test file based on its path/name.
    pub is_test_file: bool,
    /// List of line numbers that contain test functions or fixtures.
    /// Definitions on these lines will receive a confidence penalty (likely ignored).
    pub test_decorated_lines: Vec<usize>,
    /// Helper for mapping byte offsets to line numbers.
    pub line_index: &'a LineIndex,
}

impl<'a> TestAwareVisitor<'a> {
    /// Creates a new `TestAwareVisitor`.
    ///
    /// Determines if the file is a test file based on the file path.
    pub fn new(path: &Path, line_index: &'a LineIndex) -> Self {
        let path_str = path.to_string_lossy();
        // Check if the file path matches the test file regex.
        let is_test_file = crate::utils::is_test_path(&path_str);

        Self {
            is_test_file,
            test_decorated_lines: Vec::new(),
            line_index,
        }
    }

    /// Visits statements to find test functions and classes.
    pub fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::FunctionDef(node) => {
                let name = &node.name;
                let line = self.line_index.line_index(node.range.start());

                // Heuristic: Functions starting with `test_` or ending with `_test` are likely tests.
                if TEST_METHOD_PATTERN().is_match(name) || name.ends_with("_test") {
                    self.test_decorated_lines.push(line);
                }

                // Check decorators for pytest fixtures or markers.
                for decorator in &node.decorator_list {
                    let decorator_name = match &decorator.expression {
                        Expr::Name(name_node) => name_node.id.to_string(),
                        Expr::Attribute(attr_node) => {
                            // Simplified: just check the attribute name for now, or reconstruct full name
                            // For regex matching we might need the full string e.g. "pytest.fixture"
                            // But AST gives us parts. Let's try to construct a string representation.
                            format!(
                                "{}.{}",
                                match &*attr_node.value {
                                    Expr::Name(n) => &n.id,
                                    _ => "",
                                },
                                attr_node.attr
                            )
                        }
                        Expr::Call(call_node) => match &*call_node.func {
                            Expr::Name(n) => n.id.to_string(),
                            Expr::Attribute(a) => format!(
                                "{}.{}",
                                match &*a.value {
                                    Expr::Name(n) => &n.id,
                                    _ => "",
                                },
                                a.attr
                            ),
                            _ => String::new(),
                        },
                        _ => String::new(),
                    };

                    if TEST_DECOR_RE().is_match(&decorator_name) {
                        self.test_decorated_lines.push(line);
                    }
                }

                // Recurse into the function body.
                for stmt in &node.body {
                    self.visit_stmt(stmt);
                }
            }
            Stmt::ClassDef(node) => {
                let name = &node.name;
                // Heuristic: Classes named `Test...` or `...Test` are likely test suites.
                if name.starts_with("Test") || name.ends_with("Test") {
                    let line = self.line_index.line_index(node.range.start());
                    self.test_decorated_lines.push(line);
                }
                // Recurse into the class body.
                for stmt in &node.body {
                    self.visit_stmt(stmt);
                }
            }
            _ => {}
        }
    }
}
