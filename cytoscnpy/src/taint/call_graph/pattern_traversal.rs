use super::CallGraph;
use ruff_python_ast as ast;

pub(super) fn visit_pattern_for_calls(
    graph: &mut CallGraph,
    pattern: &ast::Pattern,
    caller: &str,
    module_name: &str,
) {
    if visit_value_or_class_pattern(graph, pattern, caller, module_name) {
        return;
    }
    if visit_collection_pattern(graph, pattern, caller, module_name) {
        return;
    }
    let _ = visit_alias_pattern(graph, pattern, caller, module_name);
}

fn visit_value_or_class_pattern(
    graph: &mut CallGraph,
    pattern: &ast::Pattern,
    caller: &str,
    module_name: &str,
) -> bool {
    match pattern {
        ast::Pattern::MatchValue(node) => {
            graph.visit_expr_for_calls(&node.value, caller, module_name);
        }
        ast::Pattern::MatchClass(node) => {
            graph.visit_expr_for_calls(&node.cls, caller, module_name);
            visit_patterns(graph, &node.arguments.patterns, caller, module_name);
            for keyword in &node.arguments.keywords {
                graph.visit_pattern_for_calls(&keyword.pattern, caller, module_name);
            }
        }
        _ => return false,
    }
    true
}

fn visit_collection_pattern(
    graph: &mut CallGraph,
    pattern: &ast::Pattern,
    caller: &str,
    module_name: &str,
) -> bool {
    match pattern {
        ast::Pattern::MatchMapping(node) => visit_mapping_pattern(graph, node, caller, module_name),
        ast::Pattern::MatchSequence(node) => {
            visit_patterns(graph, &node.patterns, caller, module_name);
        }
        ast::Pattern::MatchOr(node) => visit_patterns(graph, &node.patterns, caller, module_name),
        _ => return false,
    }
    true
}

fn visit_mapping_pattern(
    graph: &mut CallGraph,
    node: &ast::PatternMatchMapping,
    caller: &str,
    module_name: &str,
) {
    for key in &node.keys {
        graph.visit_expr_for_calls(key, caller, module_name);
    }
    visit_patterns(graph, &node.patterns, caller, module_name);
}

fn visit_alias_pattern(
    graph: &mut CallGraph,
    pattern: &ast::Pattern,
    caller: &str,
    module_name: &str,
) -> bool {
    let ast::Pattern::MatchAs(node) = pattern else {
        return false;
    };

    if let Some(pattern) = &node.pattern {
        graph.visit_pattern_for_calls(pattern, caller, module_name);
    }
    true
}

fn visit_patterns(
    graph: &mut CallGraph,
    patterns: &[ast::Pattern],
    caller: &str,
    module_name: &str,
) {
    for pattern in patterns {
        graph.visit_pattern_for_calls(pattern, caller, module_name);
    }
}
