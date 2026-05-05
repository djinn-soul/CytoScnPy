use super::utils::{create_finding, get_call_name};
use crate::rules::ids;
use crate::rules::{Context, Finding, Rule, RuleMetadata};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

/// Rule for detecting weak hashing algorithms in `hashlib`.
pub const META_MD5: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_MD5,
    category: super::CAT_CRYPTO,
};
/// Rule for detecting weak hashing algorithms (SHA1).
pub const META_SHA1: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_SHA1,
    category: super::CAT_CRYPTO,
};
/// Rule for detecting use of insecure ciphers.
pub const META_CIPHER: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_CIPHER,
    category: super::CAT_CRYPTO,
};
/// Rule for detecting use of insecure cipher modes.
pub const META_MODE: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_MODE,
    category: super::CAT_CRYPTO,
};
/// Rule for detecting weak pseudo-random number generators.
pub const META_RANDOM: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_RANDOM,
    category: super::CAT_CRYPTO,
};
/// Rule for detecting ``PyNaCl`` low-level binding usage.
pub const META_PYNACL_LOWLEVEL: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_PYNACL_LOWLEVEL,
    category: super::CAT_CRYPTO,
};

/// Rule for detecting weak hashing algorithms in `hashlib`.
pub struct HashlibRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}
impl HashlibRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for HashlibRule {
    fn name(&self) -> &'static str {
        "HashlibRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        // use crate::rules::danger::{META_MD5, META_SHA1};

        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                // Primary: hashlib calls
                if name == "hashlib.md5" {
                    return Some(vec![create_finding(
                        "Weak hashing algorithm (MD5)",
                        META_MD5,
                        context,
                        call.range().start(),
                        "MEDIUM",
                    )]);
                }
                if name == "hashlib.sha1" {
                    return Some(vec![create_finding(
                        "Weak hashing algorithm (SHA1)",
                        META_SHA1,
                        context,
                        call.range().start(),
                        "MEDIUM",
                    )]);
                }
                if name == "hashlib.new" {
                    if let Some(Expr::StringLiteral(s)) = call.arguments.args.first() {
                        let algo = s.value.to_string().to_lowercase();
                        if matches!(algo.as_str(), "md4" | "md5") {
                            return Some(vec![create_finding(
                                &format!("Use of insecure hash algorithm in hashlib.new: {algo}."),
                                META_MD5,
                                context,
                                call.range().start(),
                                "MEDIUM",
                            )]);
                        } else if algo == "sha1" {
                            return Some(vec![create_finding(
                                &format!("Use of insecure hash algorithm in hashlib.new: {algo}."),
                                META_SHA1,
                                context,
                                call.range().start(),
                                "MEDIUM",
                            )]);
                        }
                    }
                }
                // Secondary: Other common hashing libraries (e.g. cryptography)
                if (name.contains("Hash.MD") || name.contains("hashes.MD5"))
                    && !name.starts_with("hashlib.")
                {
                    return Some(vec![create_finding(
                        "Use of insecure MD2, MD4, or MD5 hash function.",
                        META_MD5,
                        context,
                        call.range().start(),
                        "MEDIUM",
                    )]);
                }
                if name.contains("hashes.SHA1") && !name.starts_with("hashlib.") {
                    return Some(vec![create_finding(
                        "Use of insecure SHA1 hash function.",
                        META_SHA1,
                        context,
                        call.range().start(),
                        "MEDIUM",
                    )]);
                }
            }
        }
        None
    }
}

/// Rule for detecting weak pseudo-random number generators in `random`.
pub struct RandomRule {
    /// The rule's metadata.
    pub metadata: RuleMetadata,
}
impl RandomRule {
    /// Creates a new instance with the specified metadata.
    #[must_use]
    pub fn new(metadata: RuleMetadata) -> Self {
        Self { metadata }
    }
}
impl Rule for RandomRule {
    fn name(&self) -> &'static str {
        "RandomRule"
    }
    fn metadata(&self) -> RuleMetadata {
        self.metadata
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name.starts_with("random.") {
                    let method = name.trim_start_matches("random.");
                    if matches!(
                        method,
                        "Random"
                            | "random"
                            | "randrange"
                            | "randint"
                            | "choice"
                            | "choices"
                            | "uniform"
                            | "triangular"
                            | "randbytes"
                            | "sample"
                            | "getrandbits"
                    ) {
                        return Some(vec![create_finding(
                            "Standard pseudo-random generators are not suitable for security/cryptographic purposes.",
                            self.metadata,
                            context,
                            call.range().start(),
                            "LOW",
                        )]);
                    }
                }
            }
        }
        None
    }
}

/// Check for insecure ciphers and cipher modes (B304, B305)
pub fn check_ciphers_and_modes(
    name: &str,
    call: &ast::ExprCall,
    context: &Context,
) -> Option<Finding> {
    // use crate::rules::danger::{META_CIPHER, META_MODE};

    // B304: Ciphers
    if name.contains("Cipher.ARC2")
        || name.contains("Cipher.ARC4")
        || name.contains("Cipher.Blowfish")
        || name.contains("Cipher.DES")
        || name.contains("Cipher.XOR")
        || name.contains("Cipher.TripleDES")
        || name.contains("algorithms.ARC4")
        || name.contains("algorithms.Blowfish")
    {
        return Some(create_finding(
            "Use of insecure cipher. Replace with AES.",
            META_CIPHER,
            context,
            call.range().start(),
            "HIGH",
        ));
    }
    // B305: Cipher modes
    if name.ends_with("modes.ECB") {
        return Some(create_finding(
            "Use of insecure cipher mode ECB.",
            META_MODE,
            context,
            call.range().start(),
            "MEDIUM",
        ));
    }
    None
}

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

        let is_nacl_lowlevel = name_opt.as_deref().is_some_and(|name| {
            name.contains("nacl.bindings")
                || name.starts_with("bindings.")
                || name.contains("crypto_secretbox_xsalsa20poly1305")
                || name.contains("crypto_box_curve25519xsalsa20poly1305")
                || name.contains("crypto_sign_ed25519")
                || name.contains("crypto_hash_sha256")
                || name.contains("crypto_hash_sha512")
                // Raw binding function prefix patterns
                || name.starts_with("crypto_secretbox")
                || name.starts_with("crypto_box")
                || name.starts_with("crypto_sign")
                || name.starts_with("crypto_auth")
                || name.starts_with("crypto_stream")
                || name.starts_with("crypto_onetimeauth")
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
