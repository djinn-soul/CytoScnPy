use ruff_python_ast as ast;

use super::qualnames::{ImportAliases, QualifiedName};
use super::types::SubtreeNode;

/// Schema base names that are distinctive on their own, even unqualified.
const DISTINCT_SCHEMA_BASES: &[&str] = &[
    // Pydantic / SQLModel
    "BaseModel",
    "BaseSettings",
    "SQLModel",
    // Django forms / serializers / admin
    "ModelForm",
    "ModelSerializer",
    "ModelAdmin",
    // stdlib
    "TypedDict",
    "NamedTuple",
    // SQLAlchemy
    "DeclarativeBase",
];

/// Generic base names that only count as schemas when qualified by a known
/// framework module (`models.Model`, `msgspec.Struct`, `forms.Form`), so a
/// project's own `Base` or `keras.Model` is not mistaken for one.
const QUALIFIED_SCHEMA_BASES: &[&str] = &[
    "Model",
    "Form",
    "Serializer",
    "Struct",
    "Document",
    "EmbeddedDocument",
    "Schema",
];

/// Framework modules that provide the generic schema bases above.
const SCHEMA_MODULES: &[&str] = &[
    "pydantic",
    "pydantic_settings",
    "sqlmodel",
    "django",
    "models",
    "forms",
    "serializers",
    "rest_framework",
    "admin",
    "msgspec",
    "mongoengine",
    "odmantic",
    "marshmallow",
    "ma",
    "db",
    "typing",
    "typing_extensions",
];

/// Suffixes of project-local schema bases (e.g. `AppBaseModel`, `UserSchema`).
const SCHEMA_BASE_SUFFIXES: &[&str] = &["BaseModel", "Schema", "Serializer"];

/// attrs decorators (`attrs.define`, `attr.s`, ...), matched only when they
/// resolve to the `attr`/`attrs` package.
const ATTRS_DECORATORS: &[&str] = &["s", "attrs", "define", "frozen", "mutable", "dataclass"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum SchemaKind {
    /// Declared via a base class: plain assignments (Django fields) count too.
    Base,
    /// Declared via a decorator: only annotated assignments are fields.
    Decorator,
}

/// Marks field declarations in schema/model classes as semantic so that two
/// models with different field names are not reported as clones.
pub(super) fn preserve_field_semantics(
    class: &ast::StmtClassDef,
    imports: &ImportAliases,
    body_nodes: &mut [SubtreeNode],
) {
    let Some(kind) = schema_kind(class, imports) else {
        return;
    };

    for (statement, node) in class.body.iter().zip(body_nodes) {
        if !is_field_declaration(statement, kind) {
            continue;
        }

        node.label = Some(node.signature());
        node.children.clear();
        "schema_field".clone_into(&mut node.kind);
    }
}

fn schema_kind(class: &ast::StmtClassDef, imports: &ImportAliases) -> Option<SchemaKind> {
    let resolve = |expr| imports.resolve(expr);
    if class
        .bases()
        .iter()
        .filter_map(resolve)
        .any(|b| is_schema_base(&b))
    {
        return Some(SchemaKind::Base);
    }
    class
        .decorator_list
        .iter()
        .filter_map(|d| resolve(&d.expression))
        .any(|d| is_schema_decorator(&d))
        .then_some(SchemaKind::Decorator)
}

fn is_schema_decorator(decorator: &QualifiedName) -> bool {
    let name = decorator.name();
    name == "dataclass"
        || (decorator.is_from(&["attr", "attrs"]) && ATTRS_DECORATORS.contains(&name))
}

fn is_schema_base(base: &QualifiedName) -> bool {
    let name = base.name();
    DISTINCT_SCHEMA_BASES.contains(&name)
        || (QUALIFIED_SCHEMA_BASES.contains(&name) && base.is_from(SCHEMA_MODULES))
        || SCHEMA_BASE_SUFFIXES
            .iter()
            .any(|suffix| name.len() > suffix.len() && name.ends_with(suffix))
}

/// Names that are class-level config/internals, not data fields.
const CONFIG_ATTR_NAMES: &[&str] = &[
    // Pydantic v2
    "model_config",
    "model_fields",
    "model_computed_fields",
    "model_post_init",
    // Django
    "objects",
    "DoesNotExist",
    "MultipleObjectsReturned",
    "Meta",
    "Config",
    // SQLAlchemy
    "__tablename__",
    "__table_args__",
    "__mapper_args__",
];

/// A field declaration is an annotated assignment (or, for base-class
/// schemas, a plain assignment) that represents data, not class
/// configuration. Excludes:
/// - `ClassVar[...]` annotations (standard non-field marker)
/// - Known config attribute names (e.g. `model_config`, `objects`, `Meta`)
/// - Private/dunder names (e.g. `_cache`, `__tablename__`)
fn is_field_declaration(statement: &ast::Stmt, kind: SchemaKind) -> bool {
    match statement {
        ast::Stmt::AnnAssign(assign) => {
            if is_classvar_annotation(&assign.annotation) {
                return false;
            }
            if let Some(name) = target_name(&assign.target) {
                if is_non_field_name(name) {
                    return false;
                }
            }
            true
        }
        ast::Stmt::Assign(assign) => {
            if kind != SchemaKind::Base || assign.targets.len() != 1 {
                return false;
            }
            let Some(name) = target_name(&assign.targets[0]) else {
                return false;
            };
            !is_non_field_name(name)
        }
        _ => false,
    }
}

fn is_non_field_name(name: &str) -> bool {
    CONFIG_ATTR_NAMES.contains(&name)
        || name.starts_with('_')
        || (name.starts_with("model_") && name != "model_id")
}

/// Returns the simple name of an assignment target, if it is a plain identifier.
fn target_name(target: &ast::Expr) -> Option<&str> {
    match target {
        ast::Expr::Name(name) => Some(name.id.as_str()),
        _ => None,
    }
}

/// Checks whether an annotation is `ClassVar` or `ClassVar[T]`, including
/// the qualified form `typing.ClassVar`.
fn is_classvar_annotation(annotation: &ast::Expr) -> bool {
    match annotation {
        ast::Expr::Name(name) => name.id.as_str() == "ClassVar",
        ast::Expr::Attribute(attr) => attr.attr.as_str() == "ClassVar",
        ast::Expr::Subscript(sub) => is_classvar_annotation(&sub.value),
        _ => false,
    }
}
