use super::utils::{create_finding, get_call_name, is_arg_literal, is_literal, is_literal_expr};
use crate::rules::ids;
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::Expr;
use ruff_text_size::Ranged;

#[path = "filesystem_permissions.rs"]
mod permissions;
pub use permissions::{BadFilePermissionsRule, TempfileRule};

#[path = "filesystem_race.rs"]
mod race;
pub use race::RaceConditionRule;

/// Rule for detecting potential path traversal vulnerabilities.
pub const META_PATH_TRAVERSAL: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_PATH_TRAVERSAL,
    category: super::CAT_FILESYSTEM,
};
/// Rule for detecting potentially dangerous tarfile extraction.
pub const META_TARFILE: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_TARFILE,
    category: super::CAT_FILESYSTEM,
};
/// Rule for detecting potentially dangerous zipfile extraction.
pub const META_ZIPFILE: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_ZIPFILE,
    category: super::CAT_FILESYSTEM,
};
/// Rule for detecting insecure use of temporary files.
pub const META_TEMPFILE: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_TEMPFILE,
    category: super::CAT_FILESYSTEM,
};
/// Rule for detecting insecure file permissions.
pub const META_PERMISSIONS: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_PERMISSIONS,
    category: super::CAT_FILESYSTEM,
};
/// Rule for detecting insecure usage of `tempnam` or `tmpnam`.
pub const META_TEMPNAM: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_TEMPNAM,
    category: super::CAT_FILESYSTEM,
};
/// Rule for detecting TOCTOU race conditions in filesystem operations.
pub const META_RACE_CONDITION: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_RACE_CONDITION,
    category: super::CAT_FILESYSTEM,
};

/// Rule for detecting potential path traversal vulnerabilities.
pub struct PathTraversalRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}
impl PathTraversalRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for PathTraversalRule {
    fn name(&self) -> &'static str {
        "PathTraversalRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "open"
                    || name == "os.open"
                    || name.starts_with("os.path.")
                    || name.starts_with("shutil.")
                    || name == "pathlib.Path"
                    || name == "pathlib.PurePath"
                    || name == "pathlib.PosixPath"
                    || name == "pathlib.WindowsPath"
                    || name == "Path"
                    || name == "PurePath"
                    || name == "PosixPath"
                    || name == "WindowsPath"
                    || name == "zipfile.Path"
                {
                    let is_dynamic_args = if name == "open" || name == "os.open" {
                        !is_arg_literal(&call.arguments.args, 0)
                    } else if name.starts_with("pathlib.")
                        || name == "Path"
                        || name == "PurePath"
                        || name == "PosixPath"
                        || name == "WindowsPath"
                    {
                        // For Path constructors, multiple positional args can be paths (traversal risk)
                        !is_literal(&call.arguments.args)
                    } else {
                        // For os.path.join and shutil functions, multiple positional args can be paths
                        !is_literal(&call.arguments.args)
                    };

                    let is_dynamic_kwargs = call.arguments.keywords.iter().any(|kw| {
                        kw.arg.as_ref().is_some_and(|a| {
                            let s = a.as_str();
                            s == "path"
                                || s == "file"
                                || s == "at"
                                || s == "filename"
                                || s == "filepath"
                                || s == "member"
                        }) && !is_literal_expr(&kw.value)
                    });

                    if is_dynamic_args || is_dynamic_kwargs {
                        return Some(vec![create_finding(
                            "Potential path traversal (dynamic file path)",
                            self.metadata,
                            context,
                            call.range().start(),
                            "HIGH",
                        )]);
                    }
                }
            }
        }
        None
    }
}

/// Rule for detecting potential path traversal during tarfile extraction.
pub struct TarfileExtractionRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}
impl TarfileExtractionRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for TarfileExtractionRule {
    fn name(&self) -> &'static str {
        "TarfileExtractionRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            let name_opt = get_call_name(&call.func);
            let attr_name = if let Expr::Attribute(attr) = &*call.func {
                Some(attr.attr.as_str())
            } else {
                None
            };

            let is_extraction = if let Some(name) = &name_opt {
                name.ends_with(".extractall") || name.ends_with(".extract")
            } else if let Some(attr) = attr_name {
                attr == "extractall" || attr == "extract"
            } else {
                false
            };

            if is_extraction {
                // Heuristic: check if receiver looks like a tarfile
                let mut severity = "MEDIUM";

                if let Expr::Attribute(attr) = &*call.func {
                    if crate::rules::danger::utils::is_likely_tarfile_receiver(&attr.value) {
                        severity = "HIGH";
                    }
                }

                // If it's likely a zip, we don't flag as tar HIGH (Zip rule will handle it)
                if let Expr::Attribute(attr) = &*call.func {
                    if crate::rules::danger::utils::is_likely_zipfile_receiver(&attr.value) {
                        return None; // Let ZipfileExtractionRule handle it
                    }
                }

                // Check for 'filter' argument (Python 3.12+)
                for keyword in &call.arguments.keywords {
                    if let Some(arg) = &keyword.arg {
                        if arg.as_str() == "filter" {
                            if let Expr::StringLiteral(s) = &keyword.value {
                                let val = s.value.to_str();
                                if val == "data" || val == "tar" {
                                    return None; // Safe
                                }
                            }
                            // Non-literal filter is MEDIUM
                            severity = "MEDIUM";
                        }
                    }
                }

                return Some(vec![create_finding(
                    "Potential path traversal in tarfile extraction. Ensure the tarball is trusted or members are validated.",
                    self.metadata,
                    context,
                    call.range().start(),
                    severity,
                )]);
            }
        }
        None
    }
}

/// Rule for detecting potential path traversal during zipfile extraction.
pub struct ZipfileExtractionRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}
impl ZipfileExtractionRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for ZipfileExtractionRule {
    fn name(&self) -> &'static str {
        "ZipfileExtractionRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            let name_opt = get_call_name(&call.func);
            let attr_name = if let Expr::Attribute(attr) = &*call.func {
                Some(attr.attr.as_str())
            } else {
                None
            };

            let is_extraction = if let Some(name) = &name_opt {
                name.ends_with(".extractall") || name.ends_with(".extract")
            } else if let Some(attr) = attr_name {
                attr == "extractall" || attr == "extract"
            } else {
                false
            };

            if is_extraction {
                // Heuristic: check if receiver looks like a zipfile
                if let Expr::Attribute(attr) = &*call.func {
                    if crate::rules::danger::utils::is_likely_zipfile_receiver(&attr.value) {
                        return Some(vec![create_finding(
                            "Potential path traversal in zipfile extraction. Ensure the zipfile is trusted or members are validated.",
                            self.metadata,
                            context,
                            call.range().start(),
                            "HIGH",
                        )]);
                    }
                }
            }
        }
        None
    }
}
