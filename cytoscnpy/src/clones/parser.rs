//! Parser integration with `ruff_python_parser`.
//!
//! Extracts subtrees from Python source code for clone detection.

use crate::clones::types::CloneInstance;
use crate::clones::CloneError;
use ruff_python_ast::{self as ast, Stmt};
use ruff_python_parser::parse_module;
use ruff_text_size::Ranged;
use std::path::PathBuf;

/// A subtree extracted from source code for clone analysis
#[derive(Debug, Clone)]
pub struct Subtree {
    /// Type of node (function, class, etc.)
    pub node_type: SubtreeType,
    /// Name of the function/class (if any)
    pub name: Option<String>,
    /// Start byte offset
    pub start_byte: usize,
    /// End byte offset
    pub end_byte: usize,
    /// Start line (1-indexed)
    pub start_line: usize,
    /// End line (1-indexed)
    pub end_line: usize,
    /// Source file path
    pub file: PathBuf,
    /// Raw source slice
    pub source_slice: String,
    /// Child nodes for tree comparison
    pub children: Vec<SubtreeNode>,
}

/// Type of subtree node
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubtreeType {
    /// Regular function definition
    Function,
    /// Async function definition
    AsyncFunction,
    /// Class definition
    Class,
    /// Method within a class
    Method,
}

/// A node in the subtree (for edit distance calculation)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubtreeNode {
    /// Node kind (e.g., "if", "for", "assign", "call")
    pub kind: String,
    /// Optional label (normalized identifier)
    pub label: Option<String>,
    /// Child nodes
    pub children: Vec<SubtreeNode>,
}

impl SubtreeNode {
    /// Count total nodes in this subtree
    #[must_use]
    pub fn size(&self) -> usize {
        1 + self.children.iter().map(SubtreeNode::size).sum::<usize>()
    }
}

impl Subtree {
    /// Convert to a `CloneInstance`
    #[must_use]
    pub fn to_instance(&self) -> CloneInstance {
        use crate::clones::types::NodeKind;
        use std::hash::{Hash, Hasher};

        let mut hasher = rustc_hash::FxHasher::default();
        // Hash normalized children structure
        for child in &self.children {
            child.kind.hash(&mut hasher);
        }

        // Convert SubtreeType to NodeKind
        let node_kind = match self.node_type {
            SubtreeType::Function => NodeKind::Function,
            SubtreeType::AsyncFunction => NodeKind::AsyncFunction,
            SubtreeType::Class => NodeKind::Class,
            SubtreeType::Method => NodeKind::Method,
        };

        CloneInstance {
            file: self.file.clone(),
            start_line: self.start_line,
            end_line: self.end_line,
            start_byte: self.start_byte,
            end_byte: self.end_byte,
            normalized_hash: hasher.finish(),
            name: self.name.clone(),
            node_kind,
        }
    }
}

/// AST parser for clone detection
pub struct AstParser;

impl AstParser {
    /// Parse source code and return the module
    ///
    /// # Errors
    /// Returns error if parsing fails
    pub fn parse(source: &str) -> Result<ast::ModModule, CloneError> {
        parse_module(source)
            .map(ruff_python_parser::Parsed::into_syntax)
            .map_err(|e| CloneError::ParseError(e.to_string()))
    }
}

/// Extract function and class subtrees from source code
///
/// # Errors
/// Returns error if parsing fails
pub fn extract_subtrees(source: &str, path: &PathBuf) -> Result<Vec<Subtree>, CloneError> {
    let module = AstParser::parse(source)?;
    let mut subtrees = Vec::new();

    extract_from_body(&module.body, path, source, &mut subtrees, false);

    Ok(subtrees)
}

/// Recursively extract subtrees from a statement body
fn extract_from_body(
    body: &[Stmt],
    path: &PathBuf,
    source: &str,
    subtrees: &mut Vec<Subtree>,
    in_class: bool,
) {
    for stmt in body {
        match stmt {
            Stmt::FunctionDef(f) => {
                let start_byte = f.range().start().to_usize();
                let end_byte = f.range().end().to_usize();
                let (start_line, end_line) = byte_to_lines(start_byte, end_byte, source);

                // ruff uses is_async flag instead of separate AsyncFunctionDef
                let node_type = if in_class {
                    SubtreeType::Method
                } else if f.is_async {
                    SubtreeType::AsyncFunction
                } else {
                    SubtreeType::Function
                };

                subtrees.push(Subtree {
                    node_type,
                    name: Some(f.name.to_string()),
                    start_byte,
                    end_byte,
                    start_line,
                    end_line,
                    file: path.clone(),
                    source_slice: source[start_byte..end_byte].to_string(),
                    children: extract_stmt_nodes(&f.body),
                });

                // Recurse into nested functions (reset in_class to false)
                extract_from_body(&f.body, path, source, subtrees, false);
            }
            Stmt::ClassDef(c) => {
                let start_byte = c.range().start().to_usize();
                let end_byte = c.range().end().to_usize();
                let (start_line, end_line) = byte_to_lines(start_byte, end_byte, source);

                subtrees.push(Subtree {
                    node_type: SubtreeType::Class,
                    name: Some(c.name.to_string()),
                    start_byte,
                    end_byte,
                    start_line,
                    end_line,
                    file: path.clone(),
                    source_slice: source[start_byte..end_byte].to_string(),
                    children: extract_stmt_nodes(&c.body),
                });

                // Recurse into class body for methods (set in_class to true)
                extract_from_body(&c.body, path, source, subtrees, true);
            }
            _ => {}
        }
    }
}

/// Convert byte offsets to line numbers
fn byte_to_lines(start_byte: usize, end_byte: usize, source: &str) -> (usize, usize) {
    let start_line = source[..start_byte].matches('\n').count() + 1;
    let end_line = source[..end_byte].matches('\n').count() + 1;
    (start_line, end_line)
}

/// Extract structural nodes from statements for tree comparison
fn extract_stmt_nodes(body: &[Stmt]) -> Vec<SubtreeNode> {
    body.iter().map(stmt_to_node).collect()
}

/// Convert a statement to a subtree node
#[allow(clippy::too_many_lines)]
fn stmt_to_node(stmt: &Stmt) -> SubtreeNode {
    match stmt {
        // ruff uses is_async flag, not separate AsyncFunctionDef
        Stmt::FunctionDef(f) => {
            let kind = if f.is_async {
                "async_function"
            } else {
                "function"
            };
            SubtreeNode {
                kind: kind.into(),
                label: Some(f.name.to_string()),
                children: extract_stmt_nodes(&f.body),
            }
        }
        Stmt::ClassDef(c) => SubtreeNode {
            kind: "class".into(),
            label: Some(c.name.to_string()),
            children: extract_stmt_nodes(&c.body),
        },
        Stmt::Return(r) => {
            let children = r.value.as_ref().map_or(vec![], |v| extract_expr_nodes(v));
            SubtreeNode {
                kind: "return".into(),
                label: None,
                children,
            }
        }
        Stmt::Assign(a) => {
            let mut children = vec![];
            for target in &a.targets {
                children.extend(extract_expr_nodes(target));
            }
            children.extend(extract_expr_nodes(&a.value));
            SubtreeNode {
                kind: "assign".into(),
                label: None,
                children,
            }
        }
        Stmt::AugAssign(a) => {
            let mut children = extract_expr_nodes(&a.target);
            children.extend(extract_expr_nodes(&a.value));
            SubtreeNode {
                kind: "aug_assign".into(),
                label: None, // could add op
                children,
            }
        }
        Stmt::AnnAssign(a) => {
            let mut children = extract_expr_nodes(&a.target);
            if let Some(value) = &a.value {
                children.extend(extract_expr_nodes(value));
            }
            SubtreeNode {
                kind: "ann_assign".into(),
                label: None,
                children,
            }
        }
        // ruff uses is_async flag, not separate AsyncFor
        Stmt::For(f) => {
            let kind = if f.is_async { "async_for" } else { "for" };
            let mut children = extract_expr_nodes(&f.target);
            children.extend(extract_expr_nodes(&f.iter));
            children.extend(extract_stmt_nodes(&f.body));
            SubtreeNode {
                kind: kind.into(),
                label: None,
                children,
            }
        }
        Stmt::While(w) => {
            let mut children = extract_expr_nodes(&w.test);
            children.extend(extract_stmt_nodes(&w.body));
            SubtreeNode {
                kind: "while".into(),
                label: None,
                children,
            }
        }
        Stmt::If(i) => {
            let mut children = extract_expr_nodes(&i.test);
            children.extend(extract_stmt_nodes(&i.body));
            // ruff uses elif_else_clauses (plural)
            for clause in &i.elif_else_clauses {
                if let Some(test) = &clause.test {
                    children.extend(extract_expr_nodes(test));
                }
                children.extend(extract_stmt_nodes(&clause.body));
            }
            SubtreeNode {
                kind: "if".into(),
                label: None,
                children,
            }
        }
        // ruff uses is_async flag, not separate AsyncWith
        Stmt::With(w) => {
            let kind = if w.is_async { "async_with" } else { "with" };
            let mut children = vec![];
            for item in &w.items {
                children.extend(extract_expr_nodes(&item.context_expr));
                if let Some(opt) = &item.optional_vars {
                    children.extend(extract_expr_nodes(opt));
                }
            }
            children.extend(extract_stmt_nodes(&w.body));
            SubtreeNode {
                kind: kind.into(),
                label: None,
                children,
            }
        }
        Stmt::Try(t) => {
            let mut children = extract_stmt_nodes(&t.body);
            for handler in &t.handlers {
                match handler {
                    ast::ExceptHandler::ExceptHandler(h) => {
                        if let Some(type_) = &h.type_ {
                            children.extend(extract_expr_nodes(type_));
                        }
                        children.extend(extract_stmt_nodes(&h.body));
                    }
                }
            }
            children.extend(extract_stmt_nodes(&t.orelse));
            children.extend(extract_stmt_nodes(&t.finalbody));
            SubtreeNode {
                kind: "try".into(),
                label: None,
                children,
            }
        }
        Stmt::Expr(e) => SubtreeNode {
            kind: "expr".into(),
            label: None,
            children: extract_expr_nodes(&e.value),
        },
        Stmt::Pass(_) => SubtreeNode {
            kind: "pass".into(),
            label: None,
            children: vec![],
        },
        Stmt::Break(_) => SubtreeNode {
            kind: "break".into(),
            label: None,
            children: vec![],
        },
        Stmt::Continue(_) => SubtreeNode {
            kind: "continue".into(),
            label: None,
            children: vec![],
        },
        Stmt::Raise(r) => {
            let mut children = vec![];
            if let Some(exc) = &r.exc {
                children.extend(extract_expr_nodes(exc));
            }
            if let Some(cause) = &r.cause {
                children.extend(extract_expr_nodes(cause));
            }
            SubtreeNode {
                kind: "raise".into(),
                label: None,
                children,
            }
        }
        Stmt::Assert(a) => {
            let mut children = extract_expr_nodes(&a.test);
            if let Some(msg) = &a.msg {
                children.extend(extract_expr_nodes(msg));
            }
            SubtreeNode {
                kind: "assert".into(),
                label: None,
                children,
            }
        }
        Stmt::Import(i) => {
            // simplified import handling
            let labels: Vec<String> = i.names.iter().map(|n| n.name.as_str().to_owned()).collect();
            SubtreeNode {
                kind: "import".into(),
                label: Some(labels.join(",")),
                children: vec![],
            }
        }
        Stmt::ImportFrom(i) => {
            let module = i
                .module
                .as_ref()
                .map_or("", ruff_python_ast::Identifier::as_str)
                .to_owned();
            let labels: Vec<String> = i.names.iter().map(|n| n.name.as_str().to_owned()).collect();
            SubtreeNode {
                kind: "import_from".into(),
                label: Some(format!("{}::{}", module, labels.join(","))),
                children: vec![],
            }
        }
        Stmt::Global(g) => SubtreeNode {
            kind: "global".into(),
            label: Some(
                g.names
                    .iter()
                    .map(ruff_python_ast::Identifier::as_str)
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            children: vec![],
        },
        Stmt::Nonlocal(n) => SubtreeNode {
            kind: "nonlocal".into(),
            label: Some(
                n.names
                    .iter()
                    .map(ruff_python_ast::Identifier::as_str)
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            children: vec![],
        },
        Stmt::Match(m) => SubtreeNode {
            kind: "match".into(),
            label: None,
            children: {
                let mut children = extract_expr_nodes(&m.subject);
                children.extend(m.cases.iter().flat_map(|c| extract_stmt_nodes(&c.body)));
                children
            },
        },
        Stmt::TypeAlias(t) => {
            let mut children = extract_expr_nodes(&t.name);
            children.extend(extract_expr_nodes(&t.value));
            SubtreeNode {
                kind: "type_alias".into(),
                label: None,
                children,
            }
        }
        Stmt::Delete(d) => {
            let children = d.targets.iter().flat_map(extract_expr_nodes).collect();
            SubtreeNode {
                kind: "delete".into(),
                label: None,
                children,
            }
        }
        Stmt::IpyEscapeCommand(_) => SubtreeNode {
            kind: "ipy_escape".into(),
            label: None,
            children: vec![],
        },
    }
}

/// Extract structural nodes from an expression
fn extract_expr_nodes(expr: &ast::Expr) -> Vec<SubtreeNode> {
    match expr {
        ast::Expr::Name(n) => vec![SubtreeNode {
            kind: "name".into(),
            label: Some(n.id.to_string()),
            children: vec![],
        }],
        ast::Expr::Call(c) => {
            let mut children = extract_expr_nodes(&c.func);
            for arg in &c.arguments.args {
                children.extend(extract_expr_nodes(arg));
            }
            // Ignore keywords for simplicity or add them if needed
            vec![SubtreeNode {
                kind: "call".into(),
                label: None,
                children,
            }]
        }
        ast::Expr::Attribute(a) => {
            let mut children = extract_expr_nodes(&a.value);
            children.push(SubtreeNode {
                kind: "attr".into(),
                label: Some(a.attr.to_string()),
                children: vec![],
            });
            vec![SubtreeNode {
                kind: "attribute".into(),
                label: None,
                children,
            }]
        }
        ast::Expr::BinOp(b) => {
            let mut children = extract_expr_nodes(&b.left);
            children.extend(extract_expr_nodes(&b.right));
            vec![SubtreeNode {
                kind: "bin_op".into(),
                label: None, // Could add op type here
                children,
            }]
        }
        ast::Expr::StringLiteral(s) => vec![SubtreeNode {
            kind: "str".into(),
            label: Some(s.value.to_string()),
            children: vec![],
        }],
        ast::Expr::NumberLiteral(n) => vec![SubtreeNode {
            kind: "num".into(),
            label: Some(format!("{:?}", n.value)),
            children: vec![],
        }],
        ast::Expr::BooleanLiteral(b) => vec![SubtreeNode {
            kind: "bool".into(),
            label: Some(b.value.to_string()),
            children: vec![],
        }],
        ast::Expr::NoneLiteral(_) => vec![SubtreeNode {
            kind: "none".into(),
            label: Some("None".to_owned()),
            children: vec![],
        }],
        ast::Expr::BytesLiteral(_) => vec![SubtreeNode {
            kind: "bytes".into(),
            label: Some("BYTES".to_owned()),
            children: vec![],
        }],
        ast::Expr::List(l) => {
            let children = l.elts.iter().flat_map(extract_expr_nodes).collect();
            vec![SubtreeNode {
                kind: "list".into(),
                label: None,
                children,
            }]
        }
        ast::Expr::Tuple(t) => {
            let children = t.elts.iter().flat_map(extract_expr_nodes).collect();
            vec![SubtreeNode {
                kind: "tuple".into(),
                label: None,
                children,
            }]
        }
        ast::Expr::Dict(d) => {
            let mut children = vec![];
            for item in &d.items {
                if let Some(key) = &item.key {
                    children.extend(extract_expr_nodes(key));
                }
                children.extend(extract_expr_nodes(&item.value));
            }
            vec![SubtreeNode {
                kind: "dict".into(),
                label: None,
                children,
            }]
        }
        // Fallback for other expressions
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_async_function() {
        let source = "
async def fetch_data():
    x = await api.get()
    return x
";
        let subtrees = extract_subtrees(source, &PathBuf::from("test.py")).unwrap();

        assert_eq!(subtrees.len(), 1);
        assert!(
            matches!(subtrees[0].node_type, SubtreeType::AsyncFunction),
            "Expected AsyncFunction, got {:?}",
            subtrees[0].node_type
        );
        assert_eq!(subtrees[0].name.as_deref(), Some("fetch_data"));
    }

    #[test]
    fn test_parser_nested_function() {
        let source = "
def outer():
    def inner():
        pass
    return inner
";
        let subtrees = extract_subtrees(source, &PathBuf::from("test.py")).unwrap();

        // Should capture outer and inner
        assert_eq!(subtrees.len(), 2);

        let names: Vec<&str> = subtrees.iter().filter_map(|s| s.name.as_deref()).collect();
        assert!(names.contains(&"outer"));
        assert!(names.contains(&"inner"));
    }

    #[test]
    fn test_parser_inner_class() {
        let source = "
def factory():
    class Local:
        pass
    return Local
";
        let subtrees = extract_subtrees(source, &PathBuf::from("test.py")).unwrap();

        assert_eq!(subtrees.len(), 2);
        let _names: Vec<&str> = subtrees
            .iter()
            .map(|s| s.node_type)
            .map(|t| match t {
                SubtreeType::Function => "Function",
                SubtreeType::Class => "Class",
                _ => "Other",
            })
            .collect();
        // Just checking types present
        assert!(subtrees
            .iter()
            .any(|s| s.node_type == SubtreeType::Function));
        assert!(subtrees.iter().any(|s| s.node_type == SubtreeType::Class));
    }

    #[test]
    fn test_parser_async_method() {
        let source = "
class API:
    async def get(self):
        pass
";
        let subtrees = extract_subtrees(source, &PathBuf::from("test.py")).unwrap();

        // Class + Method
        assert_eq!(subtrees.len(), 2);

        let method = subtrees
            .iter()
            .find(|s| s.name.as_deref() == Some("get"))
            .unwrap();
        assert_eq!(method.node_type, SubtreeType::Method);
    }
}
