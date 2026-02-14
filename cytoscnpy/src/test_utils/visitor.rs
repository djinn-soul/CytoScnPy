use crate::constants::{FIXTURE_DECOR_RE, TEST_DECOR_RE, TEST_METHOD_PATTERN};
use crate::utils::LineIndex;
use ruff_python_ast::{Expr, Stmt};
use std::path::Path;

/// Fixture definition metadata extracted from a file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct FixtureDefinitionHint {
    pub line: usize,
    pub function_name: String,
    pub fixture_name: String,
}

/// Static import binding for fixture resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct FixtureImportHint {
    pub local_name: String,
    pub source_module: String,
    pub source_symbol: String,
}

/// A visitor that detects test-related code.
///
/// This is important because "unused" code in test files (like helper functions or fixtures)
/// is often valid and shouldn't be reported as dead code.
#[allow(missing_docs)]
pub struct TestAwareVisitor<'a> {
    pub is_test_file: bool,
    pub test_decorated_lines: Vec<usize>,
    pub fixture_decorated_lines: Vec<usize>,
    pub fixture_names: Vec<String>,
    pub usefixtures_names: Vec<String>,
    pub fixture_definitions: Vec<FixtureDefinitionHint>,
    pub fixture_request_names: Vec<String>,
    pub fixture_imports: Vec<FixtureImportHint>,
    pub pytest_plugins: Vec<String>,
    pub line_index: &'a LineIndex,
}

#[allow(missing_docs)]
impl<'a> TestAwareVisitor<'a> {
    #[must_use]
    pub fn new(path: &Path, line_index: &'a LineIndex) -> Self {
        let path_str = path.to_string_lossy();
        let is_test_file = crate::utils::is_test_path(&path_str);

        Self {
            is_test_file,
            test_decorated_lines: Vec::new(),
            fixture_decorated_lines: Vec::new(),
            fixture_names: Vec::new(),
            usefixtures_names: Vec::new(),
            fixture_definitions: Vec::new(),
            fixture_request_names: Vec::new(),
            fixture_imports: Vec::new(),
            pytest_plugins: Vec::new(),
            line_index,
        }
    }

    pub fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::FunctionDef(node) => {
                let name = &node.name;
                // Align with definition line tracking in CytoScnPyVisitor (name token line).
                let line = self.line_index.line_index(node.name.range.start());
                let is_name_test_like =
                    TEST_METHOD_PATTERN().is_match(name) || name.ends_with("_test");

                let mut is_fixture_definition = false;
                let mut fixture_alias: Option<String> = None;
                let mut has_test_decorator = false;
                for decorator in &node.decorator_list {
                    match &decorator.expression {
                        Expr::Call(call_node) => {
                            self.extract_usefixtures(call_node);

                            if Self::is_fixture_decorator_call(call_node)
                                || Self::decorator_name_matches_fixture(&call_node.func)
                            {
                                is_fixture_definition = true;
                                if let Some(alias) = Self::extract_fixture_alias(call_node) {
                                    fixture_alias = Some(alias);
                                }
                            }

                            if Self::decorator_name_matches_test(&call_node.func) {
                                has_test_decorator = true;
                            }
                        }
                        expr => {
                            if Self::decorator_name_matches_fixture(expr) {
                                is_fixture_definition = true;
                            }
                            if Self::decorator_name_matches_test(expr) {
                                has_test_decorator = true;
                            }
                        }
                    }
                }

                if is_fixture_definition {
                    self.fixture_decorated_lines.push(line);
                    self.fixture_names.push(name.to_string());
                    let canonical_name = fixture_alias.unwrap_or_else(|| name.to_string());
                    self.fixture_definitions.push(FixtureDefinitionHint {
                        line,
                        function_name: name.to_string(),
                        fixture_name: canonical_name,
                    });
                }

                if is_name_test_like || has_test_decorator {
                    self.test_decorated_lines.push(line);
                }

                if is_fixture_definition || is_name_test_like || has_test_decorator {
                    self.extract_parameter_requests(&node.parameters);
                }

                for stmt in &node.body {
                    self.visit_stmt(stmt);
                }
            }
            Stmt::ClassDef(node) => {
                let name = &node.name;
                if name.starts_with("Test") || name.ends_with("Test") {
                    let line = self.line_index.line_index(node.name.range.start());
                    self.test_decorated_lines.push(line);
                }
                for stmt in &node.body {
                    self.visit_stmt(stmt);
                }
            }
            Stmt::Assign(node) => {
                if node.targets.iter().any(
                    |target| matches!(target, Expr::Name(name) if name.id.as_str() == "pytest_plugins"),
                ) {
                    self.extract_pytest_plugins(&node.value);
                }
            }
            Stmt::ImportFrom(node) => {
                self.extract_import_bindings(node);
            }
            _ => {}
        }
    }

    fn extract_usefixtures(&mut self, call: &ruff_python_ast::ExprCall) {
        let is_usefixtures = match &*call.func {
            Expr::Attribute(attr) => {
                let attr_name = &attr.attr;
                if attr_name == "usefixtures" {
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

        for arg in &call.arguments.args {
            if let Expr::StringLiteral(string_lit) = arg {
                let fixture_name = string_lit.value.to_string();
                if !fixture_name.is_empty() {
                    self.usefixtures_names.push(fixture_name);
                    self.fixture_request_names
                        .push(string_lit.value.to_string());
                }
            }
        }
    }

    fn extract_parameter_requests(&mut self, params: &ruff_python_ast::Parameters) {
        let mut add_param = |name: &str| {
            if name != "self" && name != "cls" {
                self.fixture_request_names.push(name.to_owned());
            }
        };

        for arg in &params.posonlyargs {
            add_param(arg.parameter.name.as_str());
        }
        for arg in &params.args {
            add_param(arg.parameter.name.as_str());
        }
        for arg in &params.kwonlyargs {
            add_param(arg.parameter.name.as_str());
        }
        if let Some(vararg) = &params.vararg {
            add_param(vararg.name.as_str());
        }
        if let Some(kwarg) = &params.kwarg {
            add_param(kwarg.name.as_str());
        }
    }

    fn extract_import_bindings(&mut self, node: &ruff_python_ast::StmtImportFrom) {
        let Some(module) = &node.module else {
            return;
        };

        for alias in &node.names {
            if alias.name.as_str() == "*" {
                continue;
            }
            let local_name = alias.asname.as_ref().unwrap_or(&alias.name).to_string();
            self.fixture_imports.push(FixtureImportHint {
                local_name,
                source_module: module.to_string(),
                source_symbol: alias.name.to_string(),
            });
        }
    }

    fn extract_pytest_plugins(&mut self, value: &Expr) {
        match value {
            Expr::StringLiteral(s) => self.pytest_plugins.push(s.value.to_string()),
            Expr::Tuple(tuple) => {
                for elt in &tuple.elts {
                    if let Expr::StringLiteral(s) = elt {
                        self.pytest_plugins.push(s.value.to_string());
                    }
                }
            }
            Expr::List(list) => {
                for elt in &list.elts {
                    if let Expr::StringLiteral(s) = elt {
                        self.pytest_plugins.push(s.value.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    fn is_fixture_decorator_call(call: &ruff_python_ast::ExprCall) -> bool {
        Self::decorator_name_matches_fixture(&call.func)
    }

    fn decorator_name_matches_fixture(expr: &Expr) -> bool {
        let name = Self::decorator_name(expr);
        !name.is_empty() && FIXTURE_DECOR_RE().is_match(&name)
    }

    fn decorator_name_matches_test(expr: &Expr) -> bool {
        let name = Self::decorator_name(expr);
        !name.is_empty() && TEST_DECOR_RE().is_match(&name)
    }

    fn decorator_name(expr: &Expr) -> String {
        match expr {
            Expr::Name(name) => name.id.to_string(),
            Expr::Attribute(attr) => match &*attr.value {
                Expr::Name(base) => format!("{}.{}", base.id, attr.attr),
                _ => attr.attr.to_string(),
            },
            _ => String::new(),
        }
    }

    fn extract_fixture_alias(call: &ruff_python_ast::ExprCall) -> Option<String> {
        for keyword in &call.arguments.keywords {
            if keyword
                .arg
                .as_ref()
                .map(ruff_python_ast::Identifier::as_str)
                != Some("name")
            {
                continue;
            }
            if let Expr::StringLiteral(s) = &keyword.value {
                let value = s.value.to_string();
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
        None
    }
}
