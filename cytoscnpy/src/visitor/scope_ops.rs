use super::*;

impl<'a> CytoScnPyVisitor<'a> {
    pub(super) fn enter_scope(&mut self, scope_type: ScopeType) {
        // Update cached prefix based on scope type
        match &scope_type {
            ScopeType::Class(name) | ScopeType::Function(name) => {
                if !self.cached_scope_prefix.is_empty() {
                    self.cached_scope_prefix.push('.');
                }
                self.cached_scope_prefix.push_str(name);
            }
            ScopeType::Module => {}
        }
        self.scope_stack.push(Scope::new(scope_type));
    }

    /// Pops the current scope from the stack and updates cached prefix.
    pub(super) fn exit_scope(&mut self) {
        if let Some(scope) = self.scope_stack.pop() {
            // Remove this scope's contribution from cached prefix
            match &scope.kind {
                ScopeType::Class(name) | ScopeType::Function(name) => {
                    // Remove ".name" or just "name" if at start
                    let name_len = name.len();
                    if self.cached_scope_prefix.len() > name_len {
                        // Has a dot before it
                        self.cached_scope_prefix
                            .truncate(self.cached_scope_prefix.len() - name_len - 1);
                    } else {
                        // It's the only thing in the prefix
                        self.cached_scope_prefix
                            .truncate(self.cached_scope_prefix.len() - name_len);
                    }
                }
                ScopeType::Module => {}
            }
        }
    }

    /// Adds a variable definition to the current scope.
    /// Maps the simple name to its fully qualified name.
    pub(super) fn add_local_def(&mut self, name: String, qualified_name: String) {
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.variables.insert(name.clone());
            scope.local_var_map.insert(name, qualified_name);
        }
    }

    /// Looks up a variable in the scope stack and returns its fully qualified name and scope index if found.
    /// Optimized: uses `cached_scope_prefix` for innermost scope to avoid rebuilding.
    pub(super) fn resolve_name_with_info(&self, name: &str) -> Option<(String, usize)> {
        let innermost_idx = self.scope_stack.len() - 1;

        for (i, scope) in self.scope_stack.iter().enumerate().rev() {
            // Class scopes are not visible to inner scopes (methods, nested classes).
            // They are only visible if they are the current (innermost) scope.
            if let ScopeType::Class(_) = &scope.kind {
                if i != innermost_idx {
                    continue;
                }
            }

            // Check local_var_map first (for function scopes with local variables)
            if let Some(qualified) = scope.local_var_map.get(name) {
                return Some((qualified.clone(), i));
            }

            // Fallback: construct qualified name if variable exists in scope
            if scope.variables.contains(name) {
                // Fast path: if this is the innermost scope, use cached prefix
                if i == innermost_idx {
                    if self.cached_scope_prefix.is_empty() {
                        return Some((name.to_owned(), i));
                    }
                    let mut result =
                        String::with_capacity(self.cached_scope_prefix.len() + 1 + name.len());
                    result.push_str(&self.cached_scope_prefix);
                    result.push('.');
                    result.push_str(name);
                    return Some((result, i));
                }

                // Slow path: build prefix up to scope at index i
                let mut total_len = name.len();
                if !self.module_name.is_empty() {
                    total_len += self.module_name.len() + 1;
                }
                for s in self.scope_stack.iter().take(i + 1).skip(1) {
                    match &s.kind {
                        ScopeType::Class(n) | ScopeType::Function(n) => {
                            total_len += n.len() + 1;
                        }
                        ScopeType::Module => {}
                    }
                }

                let mut result = String::with_capacity(total_len);
                if !self.module_name.is_empty() {
                    result.push_str(&self.module_name);
                }
                for s in self.scope_stack.iter().take(i + 1).skip(1) {
                    match &s.kind {
                        ScopeType::Class(n) | ScopeType::Function(n) => {
                            if !result.is_empty() {
                                result.push('.');
                            }
                            result.push_str(n);
                        }
                        ScopeType::Module => {}
                    }
                }
                if !result.is_empty() {
                    result.push('.');
                }
                result.push_str(name);
                return Some((result, i));
            }
        }
        None
    }

    /// Optimized: uses `cached_scope_prefix` for innermost scope to avoid rebuilding.
    pub(super) fn resolve_name(&self, name: &str) -> Option<String> {
        self.resolve_name_with_info(name).map(|(q, _)| q)
    }

    /// Records a reference to a name by incrementing its count.
    pub fn add_ref(&mut self, name: String) {
        *self.references.entry(name).or_insert(0) += 1;
    }

    /// Returns the fully qualified ID of the current scope.
    /// Used for tracking dynamic scopes.
    pub(super) fn get_current_scope_id(&self) -> String {
        if self.cached_scope_prefix.is_empty() {
            self.module_name.clone()
        } else if self.module_name.is_empty() {
            self.cached_scope_prefix.clone()
        } else {
            format!("{}.{}", self.module_name, self.cached_scope_prefix)
        }
    }

    /// Constructs a qualified name based on the current scope stack.
    /// Optimized to minimize allocations by pre-calculating capacity.
    pub(super) fn get_qualified_name(&self, name: &str) -> String {
        // Pre-calculate total length to avoid reallocations
        let mut total_len = name.len();

        if !self.module_name.is_empty() {
            total_len += self.module_name.len() + 1; // +1 for '.'
        }

        for scope in self.scope_stack.iter().skip(1) {
            match &scope.kind {
                ScopeType::Class(n) | ScopeType::Function(n) => {
                    total_len += n.len() + 1;
                }
                ScopeType::Module => {}
            }
        }

        // Build string with pre-allocated capacity
        let mut result = String::with_capacity(total_len);

        if !self.module_name.is_empty() {
            result.push_str(&self.module_name);
        }

        for scope in self.scope_stack.iter().skip(1) {
            match &scope.kind {
                ScopeType::Class(n) | ScopeType::Function(n) => {
                    if !result.is_empty() {
                        result.push('.');
                    }
                    result.push_str(n);
                }
                ScopeType::Module => {}
            }
        }

        if !result.is_empty() {
            result.push('.');
        }
        result.push_str(name);

        result
    }

    /// Visits function arguments (defaults and annotations).
    pub(super) fn visit_arguments(&mut self, args: &ast::Parameters) {
        // Visit positional-only args
        for arg in &args.posonlyargs {
            if let Some(ann) = &arg.parameter.annotation {
                self.visit_expr(ann);
            }
            if let Some(default) = &arg.default {
                self.visit_expr(default);
            }
        }
        // Visit regular args
        for arg in &args.args {
            if let Some(ann) = &arg.parameter.annotation {
                self.visit_expr(ann);
            }
            if let Some(default) = &arg.default {
                self.visit_expr(default);
            }
        }
        // Visit *args
        if let Some(arg) = &args.vararg {
            if let Some(ann) = &arg.annotation {
                self.visit_expr(ann);
            }
        }
        // Visit keyword-only args
        for arg in &args.kwonlyargs {
            if let Some(ann) = &arg.parameter.annotation {
                self.visit_expr(ann);
            }
            if let Some(default) = &arg.default {
                self.visit_expr(default);
            }
        }
        // Visit **kwargs
        if let Some(arg) = &args.kwarg {
            if let Some(ann) = &arg.annotation {
                self.visit_expr(ann);
            }
        }
    }
}
