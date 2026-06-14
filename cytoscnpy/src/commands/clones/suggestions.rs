use crate::clones::{CloneType, NodeKind};

/// Generates context-aware refactoring suggestions for clone findings.
pub(super) fn generate_clone_suggestion(
    clone_type: CloneType,
    node_kind: NodeKind,
    name: &str,
    similarity: f64,
) -> String {
    let suggestion = match clone_type {
        CloneType::Type1 => type1_suggestion(node_kind, is_init(name)),
        CloneType::Type2 => type2_suggestion(node_kind, is_special_method(name)),
        CloneType::Type3 => type3_suggestion(node_kind, name, similarity),
    };
    suggestion.to_owned()
}

fn is_init(name: &str) -> bool {
    name == "__init__"
}

fn is_special_method(name: &str) -> bool {
    is_init(name) || is_dunder(name)
}

fn is_dunder(name: &str) -> bool {
    name.starts_with("__") && name.ends_with("__")
}

fn type1_suggestion(node_kind: NodeKind, is_init: bool) -> &'static str {
    match node_kind {
        NodeKind::Class => "Remove duplicate class, import from original",
        NodeKind::Method if is_init => "Extract shared __init__ to base class",
        NodeKind::Method => "Move to base class or mixin",
        NodeKind::Function | NodeKind::AsyncFunction => {
            "Remove duplicate, import from original module"
        }
    }
}

fn type2_suggestion(node_kind: NodeKind, is_special_method: bool) -> &'static str {
    match node_kind {
        NodeKind::Class => "Consider inheritance or factory pattern",
        NodeKind::Method if is_special_method => "Extract to mixin or base class",
        NodeKind::Method => "Parameterize and move to base class",
        NodeKind::Function | NodeKind::AsyncFunction => {
            "Parameterize into single configurable function"
        }
    }
}

fn type3_suggestion(node_kind: NodeKind, name: &str, similarity: f64) -> &'static str {
    if similarity >= 0.9 {
        high_similarity_suggestion(node_kind, is_init(name))
    } else if similarity >= 0.8 {
        medium_similarity_suggestion(node_kind)
    } else {
        "Review for potential consolidation"
    }
}

fn high_similarity_suggestion(node_kind: NodeKind, is_init: bool) -> &'static str {
    match node_kind {
        NodeKind::Class => "High similarity: use inheritance",
        NodeKind::Method if is_init => "Extract common init to base class",
        NodeKind::Method => "Consider template method pattern",
        NodeKind::Function | NodeKind::AsyncFunction => {
            "Consider higher-order function or decorator"
        }
    }
}

fn medium_similarity_suggestion(node_kind: NodeKind) -> &'static str {
    match node_kind {
        NodeKind::Class => "Review for composition pattern",
        NodeKind::Method => "Consider template method pattern",
        NodeKind::Function | NodeKind::AsyncFunction => "Review for potential abstraction",
    }
}
