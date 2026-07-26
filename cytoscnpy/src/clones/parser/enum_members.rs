use ruff_python_ast as ast;

use super::types::SubtreeNode;

const ENUM_BASE_NAMES: &[&str] = &["Enum", "IntEnum", "StrEnum", "Flag", "IntFlag"];

pub(super) fn preserve_semantics(class: &ast::StmtClassDef, body_nodes: &mut [SubtreeNode]) {
    if !class.bases().iter().any(is_enum_base) {
        return;
    }

    for (statement, node) in class.body.iter().zip(body_nodes) {
        if !is_enum_member(statement) {
            continue;
        }

        node.label = Some(node_signature(node));
        "enum_member".clone_into(&mut node.kind);
    }
}

fn is_enum_base(expression: &ast::Expr) -> bool {
    let name = match expression {
        ast::Expr::Name(name) => name.id.as_str(),
        ast::Expr::Attribute(attribute) => attribute.attr.as_str(),
        _ => return false,
    };
    ENUM_BASE_NAMES.contains(&name)
}

fn is_enum_member(statement: &ast::Stmt) -> bool {
    match statement {
        ast::Stmt::Assign(_) => true,
        ast::Stmt::AnnAssign(assignment) => assignment.value.is_some(),
        _ => false,
    }
}

fn node_signature(node: &SubtreeNode) -> String {
    let mut signature = String::new();
    append_signature(node, &mut signature);
    signature
}

fn append_signature(node: &SubtreeNode, signature: &mut String) {
    signature.push_str(&node.kind.len().to_string());
    signature.push(':');
    signature.push_str(&node.kind);
    signature.push('|');

    if let Some(label) = &node.label {
        signature.push_str(&label.len().to_string());
        signature.push(':');
        signature.push_str(label);
    } else {
        signature.push('-');
    }

    signature.push('[');
    for child in &node.children {
        append_signature(child, signature);
    }
    signature.push(']');
}
