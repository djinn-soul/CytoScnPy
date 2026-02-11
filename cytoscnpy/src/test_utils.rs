use crate::utils::LineIndex;
use ruff_python_ast::{Expr, Stmt};
use std::path::Path;

use crate::constants::{FIXTURE_DECOR_RE, TEST_DECOR_RE, TEST_METHOD_PATTERN};

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
    /// List of line numbers that contain fixture definitions.
    /// These receive a softer penalty to allow for "Low Confidence" reporting.
    pub fixture_decorated_lines: Vec<usize>,

    /// Fixture names referenced via `@pytest.mark.usefixtures("name")`.
    /// These should be treated as "used" even without direct parameter reference.
    pub usefixtures_names: Vec<String>,
    /// Helper for mapping byte offsets to line numbers.
    pub line_index: &'a LineIndex,
}

impl<'a> TestAwareVisitor<'a> {
    /// Creates a new `TestAwareVisitor`.
    ///
    /// Determines if the file is a test file based on the file path.
    #[must_use]
    pub fn new(path: &Path, line_index: &'a LineIndex) -> Self {
        let path_str = path.to_string_lossy();
        // Check if the file path matches the test file regex.
        let is_test_file = crate::utils::is_test_path(&path_str);

        Self {
            is_test_file,
            test_decorated_lines: Vec::new(),
            fixture_decorated_lines: Vec::new(),
            usefixtures_names: Vec::new(),
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
                        Expr::Call(call_node) => {
                            // Check for @pytest.mark.usefixtures("fixture_name")
                            self.extract_usefixtures(call_node);
                            match &*call_node.func {
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
                            }
                        }
                        _ => String::new(),
                    };

                    if FIXTURE_DECOR_RE().is_match(&decorator_name) {
                        self.fixture_decorated_lines.push(line);
                    } else if TEST_DECOR_RE().is_match(&decorator_name) {
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

    /// Extracts fixture names from `@pytest.mark.usefixtures("name1", "name2")` decorator.
    fn extract_usefixtures(&mut self, call: &ruff_python_ast::ExprCall) {
        // Check if this is a usefixtures call: pytest.mark.usefixtures or mark.usefixtures
        let is_usefixtures = match &*call.func {
            Expr::Attribute(attr) => {
                let attr_name = &attr.attr;
                if attr_name == "usefixtures" {
                    // Check if it's pytest.mark.usefixtures or mark.usefixtures
                    match &*attr.value {
                        Expr::Attribute(inner) => inner.attr.as_str() == "mark",
                        Expr::Name(n) => n.id.as_str() == "mark",
                        _ => false,
                    }
                } else {
                    false
                }
            }
            _ => false,
        };

        if !is_usefixtures {
            return;
        }

        // Extract string arguments as fixture names
        for arg in &call.arguments.args {
            if let Expr::StringLiteral(string_lit) = arg {
                let fixture_name = string_lit.value.to_string();
                if !fixture_name.is_empty() {
                    self.usefixtures_names.push(fixture_name);
                }
            }
        }
    }
}

/// A RAII guard that restores the current working directory when dropped.
///
/// This is useful for tests that need to change the CWD but want to ensure
/// it's restored even if the test panics.
pub struct CwdGuard {
    original_cwd: std::path::PathBuf,
}

impl CwdGuard {
    /// Creates a new `CwdGuard` and changes the CWD to the specified path.
    ///
    /// # Errors
    ///
    /// Returns an error if getting the current directory or setting the new one fails.
    pub fn new<P: AsRef<std::path::Path>>(new_cwd: P) -> anyhow::Result<Self> {
        let original_cwd = std::env::current_dir()?;
        std::env::set_current_dir(new_cwd)?;
        Ok(Self { original_cwd })
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        if let Err(e) = std::env::set_current_dir(&self.original_cwd) {
            eprintln!(
                "Failed to restore CWD to {}: {}",
                self.original_cwd.display(),
                e
            );
        }
    }
}
