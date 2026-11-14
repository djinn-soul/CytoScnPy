use super::{CallGraph, CallGraphNode};
use ruff_python_ast::Stmt;
use ruff_text_size::Ranged;
use rustc_hash::FxHashSet;

pub(super) fn visit_stmt(
    graph: &mut CallGraph,
    stmt: &Stmt,
    current_func: Option<&str>,
    module_name: &str,
) {
    if visit_definition_stmt(graph, stmt, current_func, module_name) {
        return;
    }
    if visit_assignment_stmt(graph, stmt, current_func, module_name) {
        return;
    }
    if visit_simple_expr_stmt(graph, stmt, current_func, module_name) {
        return;
    }
    if super::flow_traversal::visit_branch_stmt(graph, stmt, current_func, module_name) {
        return;
    }
    if super::flow_traversal::visit_context_stmt(graph, stmt, current_func, module_name) {
        return;
    }
    let _ = super::flow_traversal::visit_exception_stmt(graph, stmt, current_func, module_name);
}

fn visit_definition_stmt(
    graph: &mut CallGraph,
    stmt: &Stmt,
    current_func: Option<&str>,
    module_name: &str,
) -> bool {
    match stmt {
        Stmt::FunctionDef(func) => {
            let func_name = graph.get_qualified_name(&func.name, module_name);
            graph
                .nodes
                .entry(func_name.clone())
                .or_insert_with(|| CallGraphNode {
                    name: func_name.clone(),
                    line: func.range().start().to_u32() as usize,
                    calls: FxHashSet::default(),
                    references: FxHashSet::default(),
                    called_by: FxHashSet::default(),
                    params: CallGraph::extract_params(&func.parameters),
                    is_root: false,
                });
            if let Some(caller) = current_func {
                for decorator in &func.decorator_list {
                    graph.visit_expr_for_calls(&decorator.expression, caller, module_name);
                }
                for default in func
                    .parameters
                    .iter_non_variadic_params()
                    .filter_map(|p| p.default())
                {
                    graph.visit_expr_for_calls(default, caller, module_name);
                }
                // A nested function runs only through the function defining it.
                // Do not link a class to its methods, otherwise all methods of a
                // reachable class would falsely be considered reachable and prevent
                // detecting genuinely unused methods.
                if !caller.ends_with("<module>") && !graph.is_current_class(caller, module_name) {
                    graph.add_reference_edge(caller, &func_name);
                }
            }
            // Record PEP 695 type parameters (bounds and defaults) as annotation references
            // so types used only as type variable bounds (e.g. `[T: TargetClass]`) stay alive.
            super::annotation_traversal::visit_type_params(
                graph,
                func.type_params.as_deref(),
                &func_name,
                module_name,
            );
            // Record types named in parameters and return type annotations as references.
            super::annotation_traversal::visit_annotation_refs(
                graph,
                func,
                &func_name,
                module_name,
            );
            visit_stmts(graph, &func.body, Some(&func_name), module_name);
        }
        Stmt::ClassDef(class) => {
            let class_name = graph.get_qualified_name(&class.name, module_name);
            // Register a node for the class definition in the call graph.
            // This allows reachability analysis to propagate alive status from the class
            // down to all types it references (such as generic base arguments or field annotations).
            graph
                .nodes
                .entry(class_name.clone())
                .or_insert_with(|| CallGraphNode {
                    name: class_name.clone(),
                    line: class.range().start().to_u32() as usize,
                    calls: FxHashSet::default(),
                    references: FxHashSet::default(),
                    called_by: FxHashSet::default(),
                    params: Vec::new(),
                    is_root: false,
                });
            if let Some(caller) = current_func {
                for decorator in &class.decorator_list {
                    graph.visit_expr_for_calls(&decorator.expression, caller, module_name);
                }
                for keyword in class.keywords() {
                    graph.visit_expr_for_calls(&keyword.value, caller, module_name);
                }
                // If this class is defined inside a function or another class, link the caller
                // to this class definition unless the caller is module level.
                if !caller.ends_with("<module>") && caller != class_name {
                    graph.add_reference_edge(caller, &class_name);
                }
            }
            // Base classes and generic type arguments (e.g. `Generic[TargetClass]`, `Base[T]`)
            // are type annotations referenced by the class. Traversing them ensures any class
            // used as a type argument in class inheritance is marked reachable.
            for base in class.bases() {
                super::annotation_traversal::visit_annotation_expr(
                    graph,
                    base,
                    &class_name,
                    module_name,
                );
            }
            // Record PEP 695 type parameter bounds/defaults for the class (e.g. `class Box[T: Target]`).
            super::annotation_traversal::visit_type_params(
                graph,
                class.type_params.as_deref(),
                &class_name,
                module_name,
            );
            graph.class_stack.push(class.name.to_string());
            // Pass class_name as the current context (`current_func`) so class-level annotated
            // attributes (e.g. `item: TargetClass` in TypedDict or regular classes) link their
            // types to this class. When the class is alive, its annotated types stay alive.
            visit_stmts(graph, &class.body, Some(&class_name), module_name);
            graph.class_stack.pop();
        }
        _ => return false,
    }
    true
}

fn visit_assignment_stmt(
    graph: &mut CallGraph,
    stmt: &Stmt,
    current_func: Option<&str>,
    module_name: &str,
) -> bool {
    let Some(caller) = current_func else {
        return false;
    };

    match stmt {
        Stmt::Assign(assign) => {
            graph.visit_expr_for_calls(&assign.value, caller, module_name);
            for target in &assign.targets {
                graph.visit_expr_for_calls(target, caller, module_name);
            }
        }
        Stmt::AugAssign(aug_assign) => {
            graph.visit_expr_for_calls(&aug_assign.value, caller, module_name);
            graph.visit_expr_for_calls(&aug_assign.target, caller, module_name);
        }
        Stmt::AnnAssign(ann_assign) => {
            // Types used in annotations (e.g. `x: TargetClass`) stay alive while the enclosing
            // function or class stays alive. This fixes false positives where classes used as
            // attribute annotations inside classes (e.g. TypedDict) were detected as unused.
            super::annotation_traversal::visit_annotation_expr(
                graph,
                &ann_assign.annotation,
                caller,
                module_name,
            );
            if let Some(value) = &ann_assign.value {
                graph.visit_expr_for_calls(value, caller, module_name);
            }
        }
        Stmt::TypeAlias(alias) => {
            // PEP 695 type alias statements `type MyAlias[T: Bound] = TargetClass`.
            // Record type parameters and the aliased value as type references of `caller`.
            super::annotation_traversal::visit_type_params(
                graph,
                alias.type_params.as_deref(),
                caller,
                module_name,
            );
            super::annotation_traversal::visit_annotation_expr(
                graph,
                &alias.value,
                caller,
                module_name,
            );
        }
        _ => return false,
    }
    true
}

fn visit_simple_expr_stmt(
    graph: &mut CallGraph,
    stmt: &Stmt,
    current_func: Option<&str>,
    module_name: &str,
) -> bool {
    let Some(caller) = current_func else {
        return false;
    };

    match stmt {
        Stmt::Expr(expr_stmt) => graph.visit_expr_for_calls(&expr_stmt.value, caller, module_name),
        Stmt::Assert(assert_stmt) => {
            graph.visit_expr_for_calls(&assert_stmt.test, caller, module_name);
            if let Some(msg) = &assert_stmt.msg {
                graph.visit_expr_for_calls(msg, caller, module_name);
            }
        }
        Stmt::Return(ret) => {
            if let Some(value) = &ret.value {
                graph.visit_expr_for_calls(value, caller, module_name);
            }
        }
        Stmt::Raise(raise_stmt) => {
            if let Some(exc) = &raise_stmt.exc {
                graph.visit_expr_for_calls(exc, caller, module_name);
            }
        }
        _ => return false,
    }
    true
}

pub(super) fn visit_stmts(
    graph: &mut CallGraph,
    stmts: &[Stmt],
    current_func: Option<&str>,
    module_name: &str,
) {
    for stmt in stmts {
        visit_stmt(graph, stmt, current_func, module_name);
    }
}
