use crate::rules::{Context, Finding};
use ruff_python_ast::Expr;

/// Message for subprocess command injection findings
pub const SUBPROCESS_INJECTION_MSG: &str =
    "Potential command injection (subprocess with shell=True and dynamic args)";

/// Extracts the name of a function or method call as a string.
pub fn get_call_name(func: &Expr) -> Option<String> {
    match func {
        Expr::Name(node) => Some(node.id.to_string()),
        Expr::Attribute(node) => {
            // Handle nested attributes: module.submodule.func
            // e.g. xml.etree.ElementTree.parse
            if let Expr::Attribute(_inner) = &*node.value {
                let prefix = get_call_name(&node.value)?;
                Some(format!("{}.{}", prefix, node.attr))
            } else if let Expr::Name(value) = &*node.value {
                Some(format!("{}.{}", value.id, node.attr))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Checks if all arguments in a list are literal values.
pub fn is_literal(args: &[Expr]) -> bool {
    args.iter().all(is_literal_expr)
}

/// Checks if a specific argument in a list (by index) is a literal value.
/// If the index is out of bounds, it returns true (assumed safe).
pub fn is_arg_literal(args: &[Expr], index: usize) -> bool {
    args.get(index).map_or(true, is_literal_expr)
}

/// Check if a single expression is a literal (constant value).
/// Returns false for dynamic values like variables, f-strings, concatenations, etc.
pub fn is_literal_expr(expr: &Expr) -> bool {
    match expr {
        Expr::StringLiteral(_)
        | Expr::BytesLiteral(_)
        | Expr::NumberLiteral(_)
        | Expr::BooleanLiteral(_)
        | Expr::NoneLiteral(_)
        | Expr::EllipsisLiteral(_) => true,
        Expr::List(list) => list.elts.iter().all(is_literal_expr),
        Expr::Tuple(tuple) => tuple.elts.iter().all(is_literal_expr),
        // f-strings, concatenations, variables, calls, etc. are NOT literal
        _ => false,
    }
}

/// Creates a security finding with the specified details.
pub fn create_finding(
    msg: &str,
    rule_id: &str,
    context: &Context,
    location: ruff_text_size::TextSize,
    severity: &str,
) -> Finding {
    let line = context.line_index.line_index(location);
    Finding {
        message: msg.to_owned(),
        rule_id: rule_id.to_owned(),
        file: context.filename.clone(),
        line,
        col: 0,
        severity: severity.to_owned(),
    }
}

/// Checks if an expression looks like it's related to tarfile operations.
/// Used to reduce false positives from unrelated .`extractall()` calls.
pub fn is_likely_tarfile_receiver(receiver: &Expr) -> bool {
    match receiver {
        // tarfile.open(...).extractall() -> receiver is Call to tarfile.open
        Expr::Call(inner_call) => {
            if let Expr::Attribute(inner_attr) = &*inner_call.func {
                // Check for tarfile.open(...)
                inner_attr.attr.as_str() == "open"
                    && matches!(&*inner_attr.value, Expr::Name(n) if n.id.as_str() == "tarfile")
            } else {
                false
            }
        }
        // Variable that might be a TarFile instance
        Expr::Name(name) => {
            let id = name.id.as_str().to_lowercase();
            id == "tarfile" || id.contains("tar") || id == "tf" || id == "t"
        }
        // Attribute access like self.tar_file or module.tar_archive
        Expr::Attribute(attr2) => {
            let attr_id = attr2.attr.as_str().to_lowercase();
            attr_id.contains("tar") || attr_id == "tf"
        }
        _ => false,
    }
}

/// Checks if an expression looks like it's related to zipfile operations.
pub fn is_likely_zipfile_receiver(receiver: &Expr) -> bool {
    match receiver {
        // zipfile.ZipFile(...).extractall() -> receiver is Call
        Expr::Call(inner_call) => {
            if let Expr::Attribute(inner_attr) = &*inner_call.func {
                // Check for zipfile.ZipFile(...)
                inner_attr.attr.as_str() == "ZipFile"
                    && matches!(&*inner_attr.value, Expr::Name(n) if n.id.as_str() == "zipfile")
            } else if let Expr::Name(name) = &*inner_call.func {
                // Direct ZipFile(...) call
                name.id.as_str() == "ZipFile"
            } else {
                false
            }
        }
        // Variable that might be a ZipFile instance
        Expr::Name(name) => {
            let id = name.id.as_str().to_lowercase();
            id == "zipfile" || id.contains("zip") || id == "zf" || id == "z"
        }
        // Attribute access like self.zip_file
        Expr::Attribute(attr2) => {
            let attr_id = attr2.attr.as_str().to_lowercase();
            attr_id.contains("zip") || attr_id == "zf"
        }
        _ => false,
    }
}

/// Checks if an expression contains references to sensitive variable names
pub fn contains_sensitive_names(expr: &Expr) -> bool {
    // Patterns must be lowercase for the check to work
    const SENSITIVE_PATTERNS: &[&str] = &[
        "password",
        "passwd",
        "pwd",
        "token",
        "secret",
        "api_key",
        "apikey",
        "api-key",
        "auth_token",
        "access_token",
        "refresh_token",
        "private_key",
        "privatekey",
        "credential",
    ];

    match expr {
        Expr::Name(name) => {
            let id = &name.id;
            // Case-insensitive substring check without allocation
            // We iterate over patterns and check if any is contained in 'id' (ignoring case)
            SENSITIVE_PATTERNS
                .iter()
                .any(|pattern| contains_ignore_case(id, pattern))
        }
        Expr::FString(fstring) => {
            // Check f-string elements for sensitive names
            for part in &fstring.value {
                if let ruff_python_ast::FStringPart::FString(f) = part {
                    for element in &f.elements {
                        if let ruff_python_ast::InterpolatedStringElement::Interpolation(interp) =
                            element
                        {
                            if contains_sensitive_names(&interp.expression) {
                                return true;
                            }
                        }
                    }
                }
            }
            false
        }
        Expr::BinOp(binop) => {
            contains_sensitive_names(&binop.left) || contains_sensitive_names(&binop.right)
        }
        Expr::Call(call) => {
            // Check arguments of nested calls
            for arg in &call.arguments.args {
                if contains_sensitive_names(arg) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

/// Checks if `haystack` contains `needle` as a substring, ignoring ASCII case.
/// `needle` must be lowercase.
pub fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    let needle_len = needle.len();
    if needle_len > haystack.len() {
        return false;
    }

    if haystack.is_ascii() {
        let haystack_bytes = haystack.as_bytes();
        let needle_bytes = needle.as_bytes();

        return haystack_bytes.windows(needle_len).any(|window| {
            window
                .iter()
                .zip(needle_bytes)
                .all(|(h, n)| h.eq_ignore_ascii_case(n))
        });
    }

    let haystack_chars: Vec<char> = haystack.chars().collect();
    let needle_chars: Vec<char> = needle.chars().collect();

    if needle_chars.len() > haystack_chars.len() {
        return false;
    }

    haystack_chars.windows(needle_chars.len()).any(|window| {
        window
            .iter()
            .zip(&needle_chars)
            .all(|(h, n)| h.to_ascii_lowercase() == *n)
    })
}
