use super::*;

impl<'a> CytoScnPyVisitor<'a> {
    pub(super) fn visit_definition_target(&mut self, target: &Expr) {
        match target {
            Expr::Name(node) => {
                let name = node.id.to_string();
                let qualified_name = self.get_qualified_name(&name);
                let (line, end_line, col, start_byte, end_byte) = self.get_range_info(node);

                self.add_definition(DefinitionInfo {
                    name: qualified_name.clone(),
                    def_type: "variable".to_owned(),
                    line,
                    end_line,
                    col,
                    start_byte,
                    end_byte,
                    full_start_byte: start_byte,
                    base_classes: smallvec::SmallVec::new(),
                });
                self.add_local_def(name, qualified_name);
            }
            Expr::Tuple(node) => {
                for elt in &node.elts {
                    self.visit_definition_target(elt);
                }
            }
            Expr::List(node) => {
                for elt in &node.elts {
                    self.visit_definition_target(elt);
                }
            }
            Expr::Starred(node) => {
                self.visit_definition_target(&node.value);
            }
            // Use visits for attribute/subscript to ensure we track usage of the object/index
            Expr::Attribute(node) => {
                self.visit_expr(&node.value);
            }
            Expr::Subscript(node) => {
                self.visit_expr(&node.value);
                self.visit_expr(&node.slice);
            }
            _ => {}
        }
    }

    /// Helper to recursively visit match patterns
    pub(super) fn visit_match_pattern(&mut self, pattern: &ast::Pattern) {
        // Recursion depth guard to prevent stack overflow on deeply nested code
        if self.depth >= MAX_RECURSION_DEPTH {
            self.recursion_limit_hit = true;
            return;
        }
        self.depth += 1;

        match pattern {
            ast::Pattern::MatchValue(node) => {
                self.visit_expr(&node.value);
            }
            ast::Pattern::MatchSingleton(_) => {
                // Literals (None, True, False) - nothing to track
            }
            ast::Pattern::MatchSequence(node) => {
                for p in &node.patterns {
                    self.visit_match_pattern(p);
                }
            }
            ast::Pattern::MatchMapping(node) => {
                for (key, value) in node.keys.iter().zip(&node.patterns) {
                    self.visit_expr(key);
                    self.visit_match_pattern(value);
                }
                if let Some(rest) = &node.rest {
                    let qualified_name = self.get_qualified_name(rest);
                    // Assuming rest identifier has range, we use node range as approximation if not available
                    // Actually rest is an Identifier which might not be Ranged directly in some AST versions,
                    // but usually String/Identifier is just string.
                    // Wait, `rest` is Identifier. In ruff_python_ast Identifier wraps string and range.
                    // But looking at code `if let Some(rest) = &node.rest`, rest type is `Identifier`.
                    // Does Identifier impl Ranged? Yes.
                    let (line, end_line, col, start_byte, end_byte) = self.get_range_info(node);
                    // Using node range because rest match captures the rest
                    self.add_definition(DefinitionInfo {
                        name: qualified_name.clone(),
                        def_type: "variable".to_owned(),
                        line,
                        end_line,
                        col,
                        start_byte,
                        end_byte,
                        full_start_byte: start_byte,
                        base_classes: smallvec::SmallVec::new(),
                    });
                    // Add to local scope so it can be resolved when used
                    self.add_local_def(rest.to_string(), qualified_name);
                }
            }
            ast::Pattern::MatchClass(node) => {
                self.visit_expr(&node.cls);
                for p in &node.arguments.patterns {
                    self.visit_match_pattern(p);
                }
                for k in &node.arguments.keywords {
                    self.visit_match_pattern(&k.pattern);
                }
            }
            ast::Pattern::MatchStar(node) => {
                if let Some(name) = &node.name {
                    let qualified_name = self.get_qualified_name(name);
                    let (line, end_line, col, start_byte, end_byte) = self.get_range_info(node);
                    self.add_definition(DefinitionInfo {
                        name: qualified_name.clone(),
                        def_type: "variable".to_owned(),
                        line,
                        end_line,
                        col,
                        start_byte,
                        end_byte,
                        full_start_byte: start_byte,
                        base_classes: smallvec::SmallVec::new(),
                    });
                    // Add to local scope so it can be resolved when used
                    self.add_local_def(name.to_string(), qualified_name);
                }
            }
            ast::Pattern::MatchAs(node) => {
                if let Some(pattern) = &node.pattern {
                    self.visit_match_pattern(pattern);
                }
                if let Some(name) = &node.name {
                    let qualified_name = self.get_qualified_name(name);
                    let (line, end_line, col, start_byte, end_byte) = self.get_range_info(node);
                    self.add_definition(DefinitionInfo {
                        name: qualified_name.clone(),
                        def_type: "variable".to_owned(),
                        line,
                        end_line,
                        col,
                        start_byte,
                        end_byte,
                        full_start_byte: start_byte,
                        base_classes: smallvec::SmallVec::new(),
                    });
                    // Add to local scope so it can be resolved when used
                    self.add_local_def(name.to_string(), qualified_name);
                }
            }
            ast::Pattern::MatchOr(node) => {
                for p in &node.patterns {
                    self.visit_match_pattern(p);
                }
            }
        }

        self.depth -= 1;
    }
}
