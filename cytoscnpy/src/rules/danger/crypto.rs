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
