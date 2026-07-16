use super::expressions::extract_expr_nodes;
use super::statements::extract_stmt_nodes;
use super::types::SubtreeNode;
use ruff_python_ast as ast;

pub(super) fn match_case_node(case: &ast::MatchCase) -> SubtreeNode {
    let mut children = vec![pattern_node(&case.pattern)];
    if let Some(guard) = &case.guard {
        children.push(SubtreeNode {
            kind: "match_guard".into(),
            label: None,
            children: extract_expr_nodes(guard),
        });
    }
    children.extend(extract_stmt_nodes(&case.body));
    SubtreeNode {
        kind: "match_case".into(),
        label: None,
        children,
    }
}

fn pattern_node(pattern: &ast::Pattern) -> SubtreeNode {
    match pattern {
        ast::Pattern::MatchValue(value) => branch("match_value", extract_expr_nodes(&value.value)),
        ast::Pattern::MatchSingleton(singleton) => SubtreeNode {
            kind: "match_singleton".into(),
            label: Some(format!("{:?}", singleton.value)),
            children: vec![],
        },
        ast::Pattern::MatchSequence(sequence) => branch(
            "match_sequence",
            sequence.patterns.iter().map(pattern_node).collect(),
        ),
        ast::Pattern::MatchMapping(mapping) => {
            let mut children = mapping
                .keys
                .iter()
                .zip(&mapping.patterns)
                .map(|(key, value)| {
                    let mut pair = extract_expr_nodes(key);
                    pair.push(pattern_node(value));
                    branch("mapping_entry", pair)
                })
                .collect::<Vec<_>>();
            if let Some(rest) = &mapping.rest {
                children.push(SubtreeNode {
                    kind: "mapping_rest".into(),
                    label: Some(rest.to_string()),
                    children: vec![],
                });
            }
            branch("match_mapping", children)
        }
        ast::Pattern::MatchClass(class) => {
            let mut children = extract_expr_nodes(&class.cls);
            children.extend(class.arguments.patterns.iter().map(pattern_node));
            children.extend(class.arguments.keywords.iter().map(|keyword| SubtreeNode {
                kind: "pattern_keyword".into(),
                label: Some(keyword.attr.to_string()),
                children: vec![pattern_node(&keyword.pattern)],
            }));
            branch("match_class", children)
        }
        ast::Pattern::MatchStar(star) => SubtreeNode {
            kind: "match_star".into(),
            label: star.name.as_ref().map(ToString::to_string),
            children: vec![],
        },
        ast::Pattern::MatchAs(alias) => SubtreeNode {
            kind: "match_as".into(),
            label: alias.name.as_ref().map(ToString::to_string),
            children: alias
                .pattern
                .as_deref()
                .map_or_else(Vec::new, |pattern| vec![pattern_node(pattern)]),
        },
        ast::Pattern::MatchOr(or_pattern) => branch(
            "match_or",
            or_pattern.patterns.iter().map(pattern_node).collect(),
        ),
    }
}

fn branch(kind: &str, children: Vec<SubtreeNode>) -> SubtreeNode {
    SubtreeNode {
        kind: kind.into(),
        label: None,
        children,
    }
}
