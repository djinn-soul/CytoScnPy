//! `PyNaCl` low-level binding detection.

use super::super::utils::{create_finding, get_call_name};
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

/// Rule for detecting `PyNaCl` low-level primitive usage (`nacl.bindings.*`).
///
/// `nacl.bindings` exposes raw C `NaCl` functions with no type safety, no nonce
/// management, and no error checking. Prefer `nacl.secret.SecretBox`,
/// `nacl.public.Box`, or `nacl.signing.SigningKey`.
/// PY205 / OWASP A02:2021.
pub struct PyNaclLowlevelRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}

impl PyNaclLowlevelRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}

impl Rule for PyNaclLowlevelRule {
    fn name(&self) -> &'static str {
        "PyNaclLowlevelRule"
    }

    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }

    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        // Detect import-based access: `from nacl import bindings` or `nacl.bindings.*`
        let Expr::Call(call) = expr else {
            return None;
        };

        let name_opt = get_call_name(&call.func);

        // Flag calls rooted at nacl.bindings, or the unambiguous full NaCl C primitive
        // names (nobody names their own function crypto_secretbox_xsalsa20poly1305).
        // Short prefixes like crypto_box/crypto_sign are omitted — too generic, high FP
        // risk on user-defined helpers (crypto_box_open, crypto_sign_verify, etc.).
        let is_nacl_lowlevel = name_opt.as_deref().is_some_and(|name| {
            name.contains("nacl.bindings")
                || name.starts_with("bindings.")
                || name.contains("crypto_secretbox_xsalsa20poly1305")
                || name.contains("crypto_box_curve25519xsalsa20poly1305")
                || name.contains("crypto_sign_ed25519")
                || name.contains("crypto_hash_sha256")
                || name.contains("crypto_hash_sha512")
        });

        if is_nacl_lowlevel {
            return Some(vec![create_finding(
                "PyNaCl low-level binding usage detected. Use high-level nacl.secret.SecretBox, nacl.public.Box, or nacl.signing.SigningKey instead of nacl.bindings.*.",
                self.metadata,
                context,
                {
                    use ruff_text_size::Ranged as _;
                    call.range().start()
                },
                "HIGH",
            )]);
        }

        None
    }

    fn enter_stmt(&mut self, stmt: &ast::Stmt, context: &Context) -> Option<Vec<Finding>> {
        // Detect `from nacl.bindings import ...` or `import nacl.bindings`
        match stmt {
            ast::Stmt::Import(import) => {
                for alias in &import.names {
                    if alias.name.as_str().contains("nacl.bindings") {
                        return Some(vec![create_finding(
                            "PyNaCl low-level binding import. Use high-level abstractions (nacl.secret, nacl.public, nacl.signing) instead.",
                            self.metadata,
                            context,
                            stmt.range().start(),
                            "HIGH",
                        )]);
                    }
                }
            }
            ast::Stmt::ImportFrom(import_from) => {
                if let Some(module) = &import_from.module {
                    if module.as_str().contains("nacl.bindings")
                        || module.as_str() == "nacl"
                            && import_from
                                .names
                                .iter()
                                .any(|a| a.name.as_str() == "bindings")
                    {
                        return Some(vec![create_finding(
                            "PyNaCl low-level binding import. Use high-level abstractions (nacl.secret, nacl.public, nacl.signing) instead.",
                            self.metadata,
                            context,
                            stmt.range().start(),
                            "HIGH",
                        )]);
                    }
                }
            }
            _ => {}
        }
        None
    }
}
