use crate::utils::LineIndex;
use ruff_python_ast::{Expr, Stmt};
use ruff_text_size::Ranged;
use rustc_hash::FxHashSet;

use super::decorators::check_decorators;
use super::django::{check_django_call_patterns, extract_urlpatterns_views};
use super::fastapi::extract_fastapi_depends;
use super::imports::get_framework_imports;

/// A visitor that detects framework usage in a Python file.
pub struct FrameworkAwareVisitor<'a> {
    /// Indicates if any known framework usage was detected in the file.
    pub is_framework_file: bool,
    /// Set of detected framework package names.
    pub detected_frameworks: FxHashSet<String>,
    /// Definition line numbers decorated with framework-specific decorators.
    pub framework_decorated_lines: FxHashSet<usize>,
    /// Helper for converting byte offsets to line numbers.
    pub line_index: &'a LineIndex,
    /// Symbol names referenced implicitly by framework conventions.
    pub framework_references: Vec<String>,
}

impl<'a> FrameworkAwareVisitor<'a> {
    /// Creates a new framework-aware visitor.
    #[must_use]
    pub fn new(line_index: &'a LineIndex) -> Self {
        Self {
            is_framework_file: false,
            detected_frameworks: FxHashSet::default(),
            framework_decorated_lines: FxHashSet::default(),
            line_index,
            framework_references: Vec::new(),
        }
    }

    /// Visits a statement and updates framework detection state.
    pub fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Import(node) => {
                for alias in &node.names {
                    let name = alias.name.as_str();
                    for framework in get_framework_imports() {
                        if name.contains(framework) {
                            self.is_framework_file = true;
                            self.detected_frameworks.insert((*framework).to_owned());
                        }
                    }
                }
            }
            Stmt::ImportFrom(node) => {
                if let Some(module) = &node.module {
                    let module_name = module.split('.').next().unwrap_or("");
                    if get_framework_imports().contains(module_name) {
                        self.is_framework_file = true;
                        self.detected_frameworks.insert(module_name.to_owned());
                    }
                }
            }
            Stmt::FunctionDef(node) => {
                let line = self.line_index.line_index(node.name.range().start());
                check_decorators(self, &node.decorator_list, line);
                extract_fastapi_depends(self, &node.parameters);
                for nested in &node.body {
                    self.visit_stmt(nested);
                }
            }
            Stmt::ClassDef(node) => {
                let mut is_framework_class = false;
                let mut is_pydantic_model = false;

                for base in node.bases() {
                    let id = match base {
                        Expr::Name(name_node) => Some(name_node.id.to_string()),
                        Expr::Attribute(attr_node) => Some(attr_node.attr.to_string()),
                        _ => None,
                    };
                    if let Some(id) = &id {
                        let id_lower = id.to_lowercase();
                        if self.is_framework_file
                            && (id_lower.contains("view")
                                || id_lower.contains("schema")
                                || id == "Model")
                        {
                            is_framework_class = true;
                            let line = self.line_index.line_index(node.name.range().start());
                            self.framework_decorated_lines.insert(line);
                        }
                        if (id == "BaseModel" || id_lower == "basemodel")
                            && self.detected_frameworks.contains("pydantic")
                        {
                            is_pydantic_model = true;
                        }
                    }
                }

                for nested in &node.body {
                    if is_framework_class {
                        if let Stmt::FunctionDef(function) = nested {
                            let line = self.line_index.line_index(function.name.range().start());
                            self.framework_decorated_lines.insert(line);
                        }
                    }
                    if is_pydantic_model {
                        if let Stmt::AnnAssign(ann) = nested {
                            if let Expr::Name(field_name) = &*ann.target {
                                self.framework_references.push(field_name.id.to_string());
                            }
                        }
                    }
                    self.visit_stmt(nested);
                }
            }
            Stmt::Assign(node) => {
                for target in &node.targets {
                    if let Expr::Name(name) = target {
                        if name.id.as_str() == "urlpatterns" {
                            self.is_framework_file = true;
                            self.detected_frameworks.insert("django".to_owned());
                            extract_urlpatterns_views(self, &node.value);
                        }
                    }
                }
            }
            Stmt::Expr(node) => {
                check_django_call_patterns(self, &node.value);
            }
            _ => {}
        }
    }
}
