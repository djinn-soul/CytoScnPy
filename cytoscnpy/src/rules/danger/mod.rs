use crate::rules::Rule;

pub mod crypto;
pub mod filesystem;
pub mod frameworks;
pub mod injection;
pub mod misc;
pub mod network;
/// Taint-aware danger rules for reduced false positives.
pub mod taint_aware;
/// Type inference rules for detecting method misuse on inferred types.
pub mod type_inference;
pub mod utils;

use crypto::{HashlibRule, RandomRule};
use filesystem::{BadFilePermissionsRule, PathTraversalRule, TempfileRule};
use frameworks::DjangoSecurityRule;
use injection::{
    AsyncSubprocessRule, EvalRule, ExecRule, ModelDeserializationRule, PickleRule,
    SqlInjectionRawRule, SqlInjectionRule, SubprocessRule, TarfileExtractionRule, XSSRule, XmlRule,
    YamlRule, ZipfileExtractionRule,
};
use misc::{
    AssertUsedRule, BlacklistCallRule, DebugModeRule, InsecureImportRule, Jinja2AutoescapeRule,
    LoggingSensitiveDataRule,
};
use network::{HardcodedBindAllInterfacesRule, RequestWithoutTimeoutRule, RequestsRule, SSRFRule};
use type_inference::MethodMisuseRule;

/// Returns a list of all security/danger rules, organized by category.
#[must_use]
pub fn get_danger_rules() -> Vec<Box<dyn Rule>> {
    vec![
        // ═══════════════════════════════════════════════════════════════════════
        // Category 1: Code Execution (CSP-D0xx) - Highest Risk
        // ═══════════════════════════════════════════════════════════════════════
        Box::new(EvalRule),       // CSP-D001: eval() usage
        Box::new(ExecRule),       // CSP-D002: exec() usage
        Box::new(SubprocessRule), // CSP-D003: Command injection
        // ═══════════════════════════════════════════════════════════════════════
        // Category 2: Injection Attacks (CSP-D1xx)
        // ═══════════════════════════════════════════════════════════════════════
        Box::new(SqlInjectionRule),    // CSP-D101: SQL injection (ORM)
        Box::new(SqlInjectionRawRule), // CSP-D102: SQL injection (raw)
        Box::new(XSSRule),             // CSP-D103: Cross-site scripting
        Box::new(XmlRule),             // CSP-D104: Insecure XML parsing (XXE/DoS)
        // ═══════════════════════════════════════════════════════════════════════
        // Category 3: Deserialization (CSP-D2xx)
        // ═══════════════════════════════════════════════════════════════════════
        Box::new(PickleRule), // CSP-D201: Pickle deserialization
        Box::new(YamlRule),   // CSP-D202: YAML unsafe load
        // ═══════════════════════════════════════════════════════════════════════
        // Category 4: Cryptography (CSP-D3xx)
        // ═══════════════════════════════════════════════════════════════════════
        Box::new(HashlibRule), // CSP-D301: Weak hash algorithms
        // ═══════════════════════════════════════════════════════════════════════
        // Category 5: Network/HTTP (CSP-D4xx)
        // ═══════════════════════════════════════════════════════════════════════
        Box::new(RequestsRule), // CSP-D401: Insecure HTTP requests
        Box::new(SSRFRule),     // CSP-D402: Server-side request forgery
        // ═══════════════════════════════════════════════════════════════════════
        // Category 6: File Operations (CSP-D5xx)
        // ═══════════════════════════════════════════════════════════════════════
        Box::new(PathTraversalRule), // CSP-D501: Path traversal attacks
        Box::new(TarfileExtractionRule), // CSP-D502: Tar extraction vulnerabilities
        Box::new(ZipfileExtractionRule), // CSP-D503: Zip extraction vulnerabilities
        Box::new(TempfileRule),      // CSP-D504: Insecure tempfile.mktemp
        Box::new(BadFilePermissionsRule), // CSP-D505: Bad file permissions
        // ═══════════════════════════════════════════════════════════════════════
        // Category X: Frameworks (CSP-D904 etc)
        // ═══════════════════════════════════════════════════════════════════════
        Box::new(DjangoSecurityRule), // CSP-D904: Django SECRET_KEY
        // ═══════════════════════════════════════════════════════════════════════
        // Category 7: Type Safety (CSP-D6xx)
        // ═══════════════════════════════════════════════════════════════════════
        Box::new(MethodMisuseRule::default()), // CSP-D601: Type-based method misuse
        // ═══════════════════════════════════════════════════════════════════════
        // Category 8: Best Practices / Misconfigurations (CSP-D7xx etc)
        // ═══════════════════════════════════════════════════════════════════════
        Box::new(AssertUsedRule), // CSP-D105: Assert used in production
        Box::new(DebugModeRule),  // CSP-D403: Debug mode enabled
        Box::new(HardcodedBindAllInterfacesRule), // CSP-D404: Binding to all interfaces
        Box::new(RequestWithoutTimeoutRule), // CSP-D405: Request without timeout
        Box::new(InsecureImportRule), // CSP-D004: Insecure imports (telnet, ftp, etc.)
        Box::new(Jinja2AutoescapeRule), // CSP-D106: Jinja2 autoescape=False
        Box::new(BlacklistCallRule), // CSP-D800: Blacklisted calls (marshal, md5, etc.)
        Box::new(RandomRule),     // CSP-D311: Weak random number generation
        // ═══════════════════════════════════════════════════════════════════════
        // Category 9: Modern Python Patterns (CSP-D9xx) - 2025/2026 Security
        // ═══════════════════════════════════════════════════════════════════════
        Box::new(AsyncSubprocessRule), // CSP-D901: Async subprocess injection
        Box::new(ModelDeserializationRule), // CSP-D902: ML model deserialization
        Box::new(LoggingSensitiveDataRule), // CSP-D903: Sensitive data in logs
    ]
}
