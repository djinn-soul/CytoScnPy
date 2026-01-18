use super::utils::{create_finding, get_call_name};
use crate::rules::{Context, Finding, Rule};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

/// Rule for detecting weak hashing algorithms in `hashlib`.
pub struct HashlibRule;
impl Rule for HashlibRule {
    fn name(&self) -> &'static str {
        "HashlibRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D301" // Default to MD5
    }
    fn visit_expr(&mut self, expr: &Expr, context: &Context) -> Option<Vec<Finding>> {
        if let Expr::Call(call) = expr {
            if let Some(name) = get_call_name(&call.func) {
                if name == "hashlib.md5" {
                    return Some(vec![create_finding(
                        "Weak hashing algorithm (MD5)",
                        "CSP-D301",
                        context,
                        call.range().start(),
                        "MEDIUM",
                    )]);
                }
                if name == "hashlib.sha1" {
                    return Some(vec![create_finding(
                        "Weak hashing algorithm (SHA1)",
                        "CSP-D302",
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
pub struct RandomRule;
impl Rule for RandomRule {
    fn name(&self) -> &'static str {
        "RandomRule"
    }
    fn code(&self) -> &'static str {
        "CSP-D311"
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
                            self.code(),
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

/// Check for marshal deserialization and insecure hash functions (B302, B303, B324)
pub fn check_marshal_and_hashes(
    name: &str,
    call: &ast::ExprCall,
    context: &Context,
) -> Option<Finding> {
    // B302: Marshal
    if name == "marshal.load" || name == "marshal.loads" {
        return Some(create_finding(
            "Deserialization with marshal is insecure.",
            "CSP-D203",
            context,
            call.range().start(),
            "MEDIUM",
        ));
    }
    // B303/B324: MD5/SHA1 (excluding hashlib as it is covered by HashlibRule)
    if (name.contains("Hash.MD") || name.contains("hashes.MD5")) && !name.starts_with("hashlib.") {
        return Some(create_finding(
            "Use of insecure MD2, MD4, or MD5 hash function.",
            "CSP-D301",
            context,
            call.range().start(),
            "MEDIUM",
        ));
    }
    if name.contains("hashes.SHA1") && !name.starts_with("hashlib.") {
        return Some(create_finding(
            "Use of insecure SHA1 hash function.",
            "CSP-D302",
            context,
            call.range().start(),
            "MEDIUM",
        ));
    }
    // B324: hashlib.new with insecure hash
    if name == "hashlib.new" {
        if let Some(Expr::StringLiteral(s)) = call.arguments.args.first() {
            let algo = s.value.to_string().to_lowercase();
            if matches!(algo.as_str(), "md4" | "md5") {
                return Some(create_finding(
                    &format!("Use of insecure hash algorithm in hashlib.new: {algo}."),
                    "CSP-D301",
                    context,
                    call.range().start(),
                    "MEDIUM",
                ));
            } else if algo == "sha1" {
                return Some(create_finding(
                    &format!("Use of insecure hash algorithm in hashlib.new: {algo}."),
                    "CSP-D302",
                    context,
                    call.range().start(),
                    "MEDIUM",
                ));
            }
        }
    }
    None
}

/// Check for insecure ciphers and cipher modes (B304, B305)
pub fn check_ciphers_and_modes(
    name: &str,
    call: &ast::ExprCall,
    context: &Context,
) -> Option<Finding> {
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
            "CSP-D304",
            context,
            call.range().start(),
            "HIGH",
        ));
    }
    // B305: Cipher modes
    if name.ends_with("modes.ECB") {
        return Some(create_finding(
            "Use of insecure cipher mode ECB.",
            "CSP-D305",
            context,
            call.range().start(),
            "MEDIUM",
        ));
    }
    None
}
