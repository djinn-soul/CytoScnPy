/// Code execution rules (eval, exec, subprocess, etc.).
pub mod code_execution;
/// Cryptography rules (weak hashes, ciphers, PRNGs).
pub mod crypto;
/// Deserialization rules (pickle, yaml, marshal).
pub mod deserialization;
/// Filesystem rules (path traversal, temp files, permissions).
pub mod filesystem;
/// Framework-specific security rules (Django, etc.).
pub mod frameworks;
/// Injection rules (`SQLi`, XSS, XML).
pub mod injection;
/// Miscellaneous rules (assert, blacklist, logging).
pub mod misc;
/// Network rules (requests, SSRF, SSL).
pub mod network;
/// Taint analysis integration.
pub mod taint_aware;
/// Type inference helpers and rules.
pub mod type_inference;
/// Utility functions for danger rules.
pub mod utils;

use crate::rules::Rule;
use code_execution::{AsyncSubprocessRule, EvalRule, ExecRule, SubprocessRule};
use crypto::{HashlibRule, RandomRule};
use deserialization::{MarshalRule, ModelDeserializationRule, PickleRule, YamlRule};
use filesystem::{
    BadFilePermissionsRule, PathTraversalRule, TarfileExtractionRule, TempfileRule,
    ZipfileExtractionRule,
};
use frameworks::DjangoSecurityRule;
use injection::{SqlInjectionRawRule, SqlInjectionRule, XSSRule, XmlRule};
use misc::{
    AssertUsedRule, BlacklistCallRule, DebugModeRule, InsecureImportRule, Jinja2AutoescapeRule,
    LoggingSensitiveDataRule,
};
use network::{HardcodedBindAllInterfacesRule, RequestWithoutTimeoutRule, RequestsRule, SSRFRule};
use std::collections::HashMap;
use type_inference::MethodMisuseRule;

// ══════════════════════════════════════════════════════════════════════════════
// Category Names (Single Source of Truth)
// ══════════════════════════════════════════════════════════════════════════════
/// Category for code execution vulnerabilities.
pub const CAT_CODE_EXEC: &str = "Code Execution";
/// Category for injection vulnerabilities (SQL, XSS, etc.).
pub const CAT_INJECTION: &str = "Injection Attacks";
/// Category for deserialization vulnerabilities.
pub const CAT_DESERIALIZATION: &str = "Deserialization";
/// Category for cryptographic issues.
pub const CAT_CRYPTO: &str = "Cryptography";
/// Category for network-related issues.
pub const CAT_NETWORK: &str = "Network & HTTP";
/// Category for filesystem operations.
pub const CAT_FILESYSTEM: &str = "File Operations";
/// Category for type safety issues.
pub const CAT_TYPE_SAFETY: &str = "Type Safety";
/// Category for general best practices.
pub const CAT_BEST_PRACTICES: &str = "Best Practices";
/// Category for privacy concerns.
pub const CAT_PRIVACY: &str = "Information Privacy";

// ══════════════════════════════════════════════════════════════════════════════
// Rule Metadata Registry
// ══════════════════════════════════════════════════════════════════════════════

/// Returns a flat list of all security rules.
#[must_use]
pub fn get_danger_rules() -> Vec<Box<dyn Rule>> {
    get_danger_rules_by_category()
        .into_iter()
        .flat_map(|(_, rules)| rules)
        .collect()
}

/// Returns security rules grouped by their functional category.
/// This preserves the intended ordering (Category 1 through 9).
#[must_use]
pub fn get_danger_rules_by_category() -> Vec<(&'static str, Vec<Box<dyn Rule>>)> {
    vec![
        (
            CAT_CODE_EXEC,
            vec![
                Box::new(EvalRule::new(code_execution::META_EVAL)),
                Box::new(ExecRule::new(code_execution::META_EXEC)),
                Box::new(SubprocessRule::new(code_execution::META_SUBPROCESS)),
                Box::new(AsyncSubprocessRule::new(
                    code_execution::META_ASYNC_SUBPROCESS,
                )),
            ],
        ),
        (
            CAT_INJECTION,
            vec![
                Box::new(SqlInjectionRule::new(injection::META_SQL_INJECTION)),
                Box::new(SqlInjectionRawRule::new(injection::META_SQL_RAW)),
                Box::new(XSSRule::new(injection::META_XSS)),
                Box::new(XmlRule::new(injection::META_XML)),
            ],
        ),
        (
            CAT_DESERIALIZATION,
            vec![
                Box::new(PickleRule::new(deserialization::META_PICKLE)),
                Box::new(YamlRule::new(deserialization::META_YAML)),
                Box::new(MarshalRule::new(deserialization::META_MARSHAL)),
                Box::new(ModelDeserializationRule::new(
                    deserialization::META_MODEL_DESER,
                )),
            ],
        ),
        (
            CAT_CRYPTO,
            vec![
                Box::new(HashlibRule::new(crypto::META_MD5)),
                Box::new(RandomRule::new(crypto::META_RANDOM)),
            ],
        ),
        (
            CAT_NETWORK,
            vec![
                Box::new(RequestsRule::new(network::META_REQUESTS)),
                Box::new(SSRFRule::new(network::META_SSRF)),
                Box::new(DebugModeRule::new(network::META_DEBUG_MODE)),
                Box::new(HardcodedBindAllInterfacesRule::new(network::META_BIND_ALL)),
                Box::new(RequestWithoutTimeoutRule::new(network::META_TIMEOUT)),
            ],
        ),
        (
            CAT_FILESYSTEM,
            vec![
                Box::new(PathTraversalRule::new(filesystem::META_PATH_TRAVERSAL)),
                Box::new(TarfileExtractionRule::new(filesystem::META_TARFILE)),
                Box::new(ZipfileExtractionRule::new(filesystem::META_ZIPFILE)),
                Box::new(TempfileRule::new(filesystem::META_TEMPFILE)),
                Box::new(BadFilePermissionsRule::new(filesystem::META_PERMISSIONS)),
            ],
        ),
        (
            CAT_TYPE_SAFETY,
            vec![Box::new(MethodMisuseRule::new(
                type_inference::META_METHOD_MISUSE,
            ))],
        ),
        (
            CAT_BEST_PRACTICES,
            vec![
                Box::new(AssertUsedRule::new(misc::META_ASSERT)),
                Box::new(InsecureImportRule::new(misc::META_INSECURE_IMPORT)),
                Box::new(Jinja2AutoescapeRule::new(misc::META_JINJA_AUTOESCAPE)),
                Box::new(BlacklistCallRule::new(misc::META_BLACKLIST)),
            ],
        ),
        (
            CAT_PRIVACY,
            vec![
                Box::new(LoggingSensitiveDataRule::new(misc::META_LOGGING_SENSITIVE)),
                Box::new(DjangoSecurityRule::new(frameworks::META_DJANGO_SECURITY)),
            ],
        ),
    ]
}

/// Returns security rules as a map for easy category-based lookup.
#[must_use]
pub fn get_danger_category_map() -> HashMap<&'static str, Vec<Box<dyn Rule>>> {
    get_danger_rules_by_category().into_iter().collect()
}
