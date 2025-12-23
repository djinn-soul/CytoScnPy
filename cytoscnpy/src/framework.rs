use crate::utils::LineIndex;
use ruff_python_ast::{Expr, Stmt};
use rustc_hash::FxHashSet;
use std::sync::OnceLock;

/// Framework-specific decorator patterns that indicate implicit usage.
/// Patterns use `@*.method` to indicate any object with that method (e.g., `@app.route`).
pub static FRAMEWORK_DECORATORS: &[&str] = &[
    "@*.route",
    "@*.get",
    "@*.post",
    "@*.put",
    "@*.delete",
    "@*.patch",
    "@*.head",
    "@*.options",
    "@login_required",
    "@permission_required",
    "@require_http_methods",
    "@csrf_exempt",
    "@api_view",
    "@action",
    "@task",
    "@shared_task",
    "@receiver",
    "@validator",
    "@root_validator",
    "@field_validator",
    "@model_validator",
    // Azure Functions v2 decorators
    "@*.function_name",
    "@*.blob_trigger",
    "@*.queue_trigger",
    "@*.timer_trigger",
    "@*.cosmos_db_trigger",
    "@*.event_hub_trigger",
    "@*.event_grid_trigger",
    "@*.service_bus_queue_trigger",
    "@*.service_bus_topic_trigger",
    "@*.blob_input",
    "@*.blob_output",
    "@*.queue_output",
    "@*.cosmos_db_input",
    "@*.cosmos_db_output",
    "@*.table_input",
    "@*.table_output",
];

/// Framework-specific function names that indicate implicit usage.
/// These are typically methods in Django views, DRF viewsets, etc.
pub static FRAMEWORK_FUNCTIONS: &[&str] = &[
    "get",
    "post",
    "put",
    "patch",
    "delete",
    "head",
    "options",
    "list",
    "create",
    "retrieve",
    "update",
    "partial_update",
    "destroy",
    "perform_create",
    "perform_update",
    "perform_destroy",
    "get_queryset",
    "get_object",
    "get_serializer",
    "get_serializer_class",
    "get_context_data",
    "get_template_name",
    "form_valid",
    "form_invalid",
    "*_queryset",
];

/// Returns the set of framework import names used for detection.
pub fn get_framework_imports() -> &'static FxHashSet<&'static str> {
    static IMPORTS: OnceLock<FxHashSet<&'static str>> = OnceLock::new();
    IMPORTS.get_or_init(|| {
        let mut s = FxHashSet::default();
        s.insert("flask");
        s.insert("fastapi");
        s.insert("django");
        s.insert("rest_framework");
        s.insert("pydantic");
        s.insert("celery");
        s.insert("starlette");
        s.insert("uvicorn");
        // Azure Functions
        s.insert("azure.functions");
        s.insert("azure_functions");
        s
    })
}

/// A visitor that detects framework usage in a Python file.
///
/// Frameworks often use decorators or inheritance to register components.
/// This visitor helps CytoScnPy understand that some code might appear unused but is actually
/// used by the framework (e.g., a route handler).
pub struct FrameworkAwareVisitor<'a> {
    /// Indicates if the current file uses a known framework.
    pub is_framework_file: bool,
    /// Set of detected frameworks in the file.
    pub detected_frameworks: FxHashSet<String>,
    /// Lines where framework-specific decorators are applied.
    /// Definitions on these lines receive a confidence penalty (are less likely to be reported as unused).
    pub framework_decorated_lines: FxHashSet<usize>,
    /// Helper for mapping byte offsets to line numbers.
    pub line_index: &'a LineIndex,
    /// Names of functions/classes referenced by framework patterns.
    /// Includes Django (urlpatterns, `admin.register`, signals), `FastAPI` (Depends), and Pydantic (`BaseModel` fields).
    /// These should be marked as "used" in the analyzer.
    pub framework_references: Vec<String>,
}

impl<'a> FrameworkAwareVisitor<'a> {
    /// Creates a new `FrameworkAwareVisitor`.
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

    /// Visits a statement to check for framework patterns.
    pub fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            // Check imports to detect framework usage.
            Stmt::Import(node) => {
                for alias in &node.names {
                    let name = alias.name.as_str();
                    // Check if the imported module is a known framework.
                    for fw in get_framework_imports() {
                        if name.contains(fw) {
                            self.is_framework_file = true;
                            self.detected_frameworks.insert((*fw).to_owned());
                        }
                    }
                }
            }
            // Check 'from ... import' statements.
            Stmt::ImportFrom(node) => {
                if let Some(module) = &node.module {
                    // Extract the base module name.
                    let module_name = module.split('.').next().unwrap_or("");
                    if get_framework_imports().contains(module_name) {
                        self.is_framework_file = true;
                        self.detected_frameworks.insert(module_name.to_owned());
                    }
                }
            }
            // Check function definitions for decorators.
            Stmt::FunctionDef(node) => {
                let line = self.line_index.line_index(node.range.start());
                self.check_decorators(&node.decorator_list, line);
                // Check for FastAPI Depends() in parameters
                self.extract_fastapi_depends(&node.parameters);
                // Recursively visit the body of the function.
                for stmt in &node.body {
                    self.visit_stmt(stmt);
                }
            }
            // Check class definitions for base classes and content.
            Stmt::ClassDef(node) => {
                let mut is_framework_class = false;
                let mut is_pydantic_model = false;
                // Check base classes (inheritance) for framework patterns.
                // e.g., inheriting from `Model`, `View`, `Schema`, `BaseModel`.
                for base in node.bases() {
                    let id = match base {
                        Expr::Name(name_node) => Some(name_node.id.to_string()),
                        Expr::Attribute(attr_node) => Some(attr_node.attr.to_string()),
                        _ => None,
                    };

                    if let Some(id) = &id {
                        let id_lower = id.to_lowercase();
                        // Only mark as framework class if we've already detected a framework import
                        // This prevents user-defined classes (like a custom BaseModel) from
                        // incorrectly triggering framework detection
                        if self.is_framework_file {
                            // Django views, schemas (serializers), etc.
                            if id_lower.contains("view") || id_lower.contains("schema") {
                                is_framework_class = true;
                                let line = self.line_index.line_index(node.range.start());
                                self.framework_decorated_lines.insert(line);
                            }
                            // Django Model (exact match, not just contains "model")
                            if id == "Model" {
                                is_framework_class = true;
                                let line = self.line_index.line_index(node.range.start());
                                self.framework_decorated_lines.insert(line);
                            }
                        }
                        // Check specifically for Pydantic BaseModel
                        // This DOES set is_framework_file because Pydantic is a real framework
                        if id == "BaseModel" || id_lower == "basemodel" {
                            // Only treat as Pydantic if we've already detected pydantic import
                            if self.detected_frameworks.contains("pydantic") {
                                is_pydantic_model = true;
                            }
                            // Note: We don't set is_framework_file or is_framework_class here
                            // for user-defined BaseModel classes
                        }
                    }
                }

                // Recursively visit the body of the class.
                for stmt in &node.body {
                    // If it's a framework class, mark its methods.
                    if is_framework_class {
                        if let Stmt::FunctionDef(f) = stmt {
                            let line = self.line_index.line_index(f.range.start());
                            self.framework_decorated_lines.insert(line);
                        }
                    }
                    // If it's a Pydantic model, mark annotated fields as used
                    if is_pydantic_model {
                        if let Stmt::AnnAssign(ann) = stmt {
                            if let Expr::Name(field_name) = &*ann.target {
                                self.framework_references.push(field_name.id.to_string());
                            }
                        }
                    }
                    self.visit_stmt(stmt);
                }
            }
            // Handle assignments - check for Django urlpatterns
            Stmt::Assign(node) => {
                // Check if this is a urlpatterns assignment
                for target in &node.targets {
                    if let Expr::Name(name) = target {
                        if name.id.as_str() == "urlpatterns" {
                            self.is_framework_file = true;
                            self.detected_frameworks.insert("django".to_owned());
                            // Extract view functions from path() and re_path() calls
                            self.extract_urlpatterns_views(&node.value);
                        }
                    }
                }
            }
            // Handle expression statements - check for admin.register() and signal.connect()
            Stmt::Expr(node) => {
                self.check_django_call_patterns(&node.value);
            }
            _ => {}
        }
    }

    /// Extracts dependency functions from `FastAPI` `Depends()` in function parameters.
    /// Example: `def get_items(db: Session = Depends(get_db))` -> marks `get_db` as used
    fn extract_fastapi_depends(&mut self, args: &ruff_python_ast::Parameters) {
        // Check all argument types for Depends() default values
        for arg in &args.args {
            if let Some(default) = &arg.default {
                self.check_depends_call(default);
            }
        }
        for arg in &args.kwonlyargs {
            if let Some(default) = &arg.default {
                self.check_depends_call(default);
            }
        }
    }

    /// Checks if an expression is a `Depends()` call and extracts the dependency function.
    fn check_depends_call(&mut self, expr: &Expr) {
        if let Expr::Call(call) = expr {
            let func_name = Self::get_call_name(&call.func);
            if func_name == "Depends" {
                self.is_framework_file = true;
                self.detected_frameworks.insert("fastapi".to_owned());
                // First argument is the dependency function
                if let Some(first_arg) = call.arguments.args.first() {
                    match first_arg {
                        Expr::Name(name) => {
                            self.framework_references.push(name.id.to_string());
                        }
                        Expr::Attribute(attr) => {
                            // Handle qualified names like module.func
                            if let Expr::Name(name) = &*attr.value {
                                self.framework_references.push(name.id.to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Extracts view function references from Django urlpatterns list.
    fn extract_urlpatterns_views(&mut self, expr: &Expr) {
        match expr {
            Expr::List(list) => {
                for elt in &list.elts {
                    self.extract_path_view(elt);
                }
            }
            Expr::BinOp(binop) => {
                // Handle urlpatterns = [...] + [...] or [...] + include(...)
                self.extract_urlpatterns_views(&binop.left);
                self.extract_urlpatterns_views(&binop.right);
            }
            _ => {}
        }
    }

    /// Extracts view function from a `path()` or `re_path()` call.
    fn extract_path_view(&mut self, expr: &Expr) {
        if let Expr::Call(call) = expr {
            let func_name = Self::get_call_name(&call.func);
            // Check for path(), re_path(), url() - Django URL routing functions
            if func_name == "path" || func_name == "re_path" || func_name == "url" {
                // Second argument is typically the view function
                if call.arguments.args.len() >= 2 {
                    self.extract_view_reference(&call.arguments.args[1]);
                }
            }
            // Check for include() - it references other URL modules, not view functions
        }
    }

    /// Extracts view reference from the second argument of `path()`.
    fn extract_view_reference(&mut self, expr: &Expr) {
        match expr {
            Expr::Name(name) => {
                // Direct function reference: path("home/", my_view)
                self.framework_references.push(name.id.to_string());
            }
            Expr::Attribute(attr) => {
                // Class-based view: path("home/", MyView.as_view())
                // Get the class name from the attribute's value
                if let Expr::Name(name) = &*attr.value {
                    self.framework_references.push(name.id.to_string());
                }
            }
            Expr::Call(call) => {
                // Could be MyView.as_view() or some wrapper
                self.extract_view_reference(&call.func);
            }
            _ => {}
        }
    }

    /// Checks for Django-specific call patterns like `admin.site.register()` and `signal.connect()`.
    fn check_django_call_patterns(&mut self, expr: &Expr) {
        if let Expr::Call(call) = expr {
            let func_name = Self::get_call_name(&call.func);

            // Check for admin.site.register(Model) or admin.register(Model)
            if func_name == "register" {
                if Self::is_admin_register(&call.func) {
                    self.is_framework_file = true;
                    self.detected_frameworks.insert("django".to_owned());
                    // First argument is the Model class
                    if let Some(Expr::Name(name)) = call.arguments.args.first() {
                        self.framework_references.push(name.id.to_string());
                    }
                }
            }
            // Check for signal.connect(receiver) - e.g., pre_save.connect(handler)
            else if func_name == "connect" && Self::is_signal_connect(&call.func) {
                self.is_framework_file = true;
                self.detected_frameworks.insert("django".to_owned());
                // First argument is the receiver function
                if let Some(Expr::Name(name)) = call.arguments.args.first() {
                    self.framework_references.push(name.id.to_string());
                }
            }
        }
    }

    /// Checks if the call is admin.site.register or admin.register.
    fn is_admin_register(func: &Expr) -> bool {
        if let Expr::Attribute(attr) = func {
            // Check for admin.site.register
            if let Expr::Attribute(inner) = &*attr.value {
                if inner.attr.as_str() == "site" {
                    if let Expr::Name(name) = &*inner.value {
                        return name.id.as_str() == "admin";
                    }
                }
            }
            // Check for admin.register (decorator style object)
            if let Expr::Name(name) = &*attr.value {
                return name.id.as_str() == "admin";
            }
        }
        false
    }

    /// Checks if the call is a Django signal connect (`pre_save.connect`, `post_save.connect`, etc.).
    fn is_signal_connect(func: &Expr) -> bool {
        if let Expr::Attribute(attr) = func {
            if let Expr::Name(name) = &*attr.value {
                let signal_names = [
                    "pre_save",
                    "post_save",
                    "pre_delete",
                    "post_delete",
                    "pre_init",
                    "post_init",
                    "m2m_changed",
                    "pre_migrate",
                    "post_migrate",
                    "request_started",
                    "request_finished",
                    "got_request_exception",
                ];
                return signal_names.contains(&name.id.as_str());
            }
        }
        false
    }

    /// Gets the name of the function being called.
    fn get_call_name(func: &Expr) -> String {
        match func {
            Expr::Name(name) => name.id.to_string(),
            Expr::Attribute(attr) => attr.attr.to_string(),
            _ => String::new(),
        }
    }

    /// Checks if any of the decorators are framework-related.
    fn check_decorators(&mut self, decorators: &[ruff_python_ast::Decorator], line: usize) {
        for decorator in decorators {
            let name = self.get_decorator_name(&decorator.expression);
            if Self::is_framework_decorator(&name) {
                // If a framework decorator is found, mark the line and the file.
                self.framework_decorated_lines.insert(line);
                self.is_framework_file = true;
            }
        }
    }

    /// Extracts the name of a decorator.
    #[allow(clippy::only_used_in_recursion)]
    fn get_decorator_name(&self, decorator: &Expr) -> String {
        match decorator {
            Expr::Name(node) => node.id.to_string(),
            Expr::Attribute(node) => {
                // For decorators like @app.route
                node.attr.to_string()
            }
            Expr::Call(node) => {
                // For decorators with arguments like @app.route("/path")
                self.get_decorator_name(&node.func)
            }
            _ => String::new(),
        }
    }

    /// Determines if a decorator name is likely framework-related.
    fn is_framework_decorator(name: &str) -> bool {
        let name = name.to_lowercase();
        // Common patterns in Flask, FastAPI, Celery, etc.
        name.contains("route")
            || name.contains("get")
            || name.contains("post")
            || name.contains("put")
            || name.contains("delete")
            || name.contains("validator")
            || name.contains("task") // celery
            || name.contains("login_required") // django
            || name.contains("permission_required") // django
            // Azure Functions v2 patterns
            || name.contains("trigger") // blob_trigger, queue_trigger, timer_trigger, etc.
            || name.contains("function_name") // @app.function_name
            || name.ends_with("_input") // blob_input, cosmos_db_input, etc.
            || name.ends_with("_output") // blob_output, queue_output, etc.
    }
}

/// Detects framework usage for a given definition.
///
/// Returns the confidence score (0-100) if the definition is a framework endpoint,
/// or `None` if it should not be flagged as framework usage.
///
/// # Arguments
/// * `line` - The line number where the definition starts
/// * `simple_name` - The simple name of the definition (e.g., "`get_users`")
/// * `def_type` - The type of definition ("function", "method", "class", "variable")
/// * `visitor` - Optional reference to the `FrameworkAwareVisitor` for the file
///
/// # Returns
/// * `Some(100)` - If the definition is a decorated framework endpoint (confidence = 1.0)
/// * `None` - If the definition is not a framework endpoint or should be ignored
#[must_use]
pub fn detect_framework_usage(
    line: usize,
    simple_name: &str,
    def_type: &str,
    visitor: Option<&FrameworkAwareVisitor>,
) -> Option<u8> {
    // No visitor means we can't determine framework usage
    let visitor = visitor?;

    // Only process functions and methods
    if def_type != "function" && def_type != "method" {
        return None;
    }

    // Non-framework files don't have framework usage
    if !visitor.is_framework_file {
        return None;
    }

    // Private functions are not framework endpoints
    if simple_name.starts_with('_') && !simple_name.starts_with("__") {
        return None;
    }

    // Check if the line is decorated with a framework decorator
    if visitor.framework_decorated_lines.contains(&line) {
        return Some(100); // confidence = 1.0 (scaled to 100)
    }

    // Undecorated functions in framework files are not flagged
    None
}
