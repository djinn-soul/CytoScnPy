use ruff_python_ast as ast;

use super::qualnames::{ImportAliases, QualifiedName};
use super::types::SubtreeNode;

/// Known stdlib and framework enum base class names.
const ENUM_BASE_NAMES: &[&str] = &[
    // stdlib
    "Enum",
    "IntEnum",
    "StrEnum",
    "Flag",
    "IntFlag",
    "ReprEnum",
    // Django
    "TextChoices",
    "IntegerChoices",
    "Choices",
];

/// Modules a qualified enum base may come from (`enum.Enum`, `models.TextChoices`).
const ENUM_MODULES: &[&str] = &["enum", "aenum", "strenum", "django", "models"];

/// Suffixes that suggest a project-local enum base (e.g. `BaseEnum`,
/// `CustomChoices`). Matches on these are heuristic, so only conventional
/// `UPPER_CASE` members are treated as enum members.
const ENUM_BASE_SUFFIXES: &[&str] = &["Enum", "Choices"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum EnumEvidence {
    Known,
    Heuristic,
}

/// True when the class derives from a recognised enum base.
pub(crate) fn is_enum_class(class: &ast::StmtClassDef, imports: &ImportAliases) -> bool {
    enum_evidence(class, imports).is_some()
}

pub(super) fn preserve_semantics(
    class: &ast::StmtClassDef,
    imports: &ImportAliases,
    body_nodes: &mut [SubtreeNode],
) {
    let Some(evidence) = enum_evidence(class, imports) else {
        return;
    };

    for (statement, node) in class.body.iter().zip(body_nodes) {
        if !is_enum_member(statement, evidence) {
            continue;
        }

        node.label = Some(node.signature());
        node.children.clear();
        "enum_member".clone_into(&mut node.kind);
    }
}

fn enum_evidence(class: &ast::StmtClassDef, imports: &ImportAliases) -> Option<EnumEvidence> {
    class
        .bases()
        .iter()
        .filter_map(|base| imports.resolve(base))
        .filter_map(|base| base_evidence(&base))
        .min_by_key(|evidence| *evidence == EnumEvidence::Heuristic)
}

fn base_evidence(base: &QualifiedName) -> Option<EnumEvidence> {
    let name = base.name();
    if ENUM_BASE_NAMES.contains(&name) && (base.is_bare() || base.is_from(ENUM_MODULES)) {
        return Some(EnumEvidence::Known);
    }
    ENUM_BASE_SUFFIXES
        .iter()
        .any(|suffix| name.len() > suffix.len() && name.ends_with(suffix))
        .then_some(EnumEvidence::Heuristic)
}

fn is_enum_member(statement: &ast::Stmt, evidence: EnumEvidence) -> bool {
    let target = match statement {
        ast::Stmt::Assign(assignment) if assignment.targets.len() == 1 => &assignment.targets[0],
        ast::Stmt::Assign(_) => return evidence == EnumEvidence::Known,
        ast::Stmt::AnnAssign(assignment) if assignment.value.is_some() => &assignment.target,
        _ => return false,
    };
    evidence == EnumEvidence::Known || is_upper_case_name(target)
}

fn is_upper_case_name(target: &ast::Expr) -> bool {
    let ast::Expr::Name(name) = target else {
        return false;
    };
    let id = name.id.as_str();
    id.chars().any(|c| c.is_ascii_uppercase())
        && id
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}
