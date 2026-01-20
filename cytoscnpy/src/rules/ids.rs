//! Centralized Rule IDs for CytoScnPy.

/// Code Execution: `eval()`
pub const RULE_ID_EVAL: &str = "CSP-D001";
/// Code Execution: `exec()` or `compile()`
pub const RULE_ID_EXEC: &str = "CSP-D002";
/// Code Execution: Command injection in `subprocess`/`os.system`
pub const RULE_ID_SUBPROCESS: &str = "CSP-D003";
/// Code Execution: Command injection in async `subprocess`/`popen`
pub const RULE_ID_ASYNC_SUBPROCESS: &str = "CSP-D004";
/// Code Execution: unsafe use of `input()`
pub const RULE_ID_INPUT: &str = "CSP-D005";

/// Injection: SQL Injection (ORM/Query builders)
pub const RULE_ID_SQL_INJECTION: &str = "CSP-D101";
/// Injection: Raw SQL string concatenation
pub const RULE_ID_SQL_RAW: &str = "CSP-D102";
/// Injection: Reflected XSS
pub const RULE_ID_XSS: &str = "CSP-D103";
/// Injection: Insecure XML parsing (XXE)
pub const RULE_ID_XML: &str = "CSP-D104";
/// Injection: `mark_safe` bypassing escaping
pub const RULE_ID_MARK_SAFE: &str = "CSP-D105";

/// Deserialization: pickle/dill/shelve
pub const RULE_ID_PICKLE: &str = "CSP-D201";
/// Deserialization: Unsafe YAML load
pub const RULE_ID_YAML: &str = "CSP-D202";
/// Deserialization: `marshal.load()`
pub const RULE_ID_MARSHAL: &str = "CSP-D203";
/// Deserialization: ML model loading (torch, keras, joblib)
pub const RULE_ID_MODEL_DESER: &str = "CSP-D204";

/// Cryptography: Weak hashing (MD5)
pub const RULE_ID_MD5: &str = "CSP-D301";
/// Cryptography: Weak hashing (SHA1)
pub const RULE_ID_SHA1: &str = "CSP-D302";
/// Cryptography: Insecure cipher
pub const RULE_ID_CIPHER: &str = "CSP-D304";
/// Cryptography: Insecure cipher mode
pub const RULE_ID_MODE: &str = "CSP-D305";
/// Cryptography: Weak PRNG
pub const RULE_ID_RANDOM: &str = "CSP-D311";

/// Network: insecure requests (verify=False)
pub const RULE_ID_REQUESTS: &str = "CSP-D401";
/// Network: Server-Side Request Forgery (SSRF)
pub const RULE_ID_SSRF: &str = "CSP-D402";
/// Network: Debug mode in production
pub const RULE_ID_DEBUG_MODE: &str = "CSP-D403";
/// Network: Hardcoded binding to 0.0.0.0
pub const RULE_ID_BIND_ALL: &str = "CSP-D404";
/// Network: Requests without timeout
pub const RULE_ID_TIMEOUT: &str = "CSP-D405";
/// Network: Insecure `FTP`
pub const RULE_ID_FTP: &str = "CSP-D406";
/// Network: `HTTPSConnection` without context
pub const RULE_ID_HTTPS_CONNECTION: &str = "CSP-D407";
/// Network: Unverified SSL context
pub const RULE_ID_SSL_UNVERIFIED: &str = "CSP-D408";
/// Network: Insecure Telnet
pub const RULE_ID_TELNET: &str = "CSP-D409";
/// Network: Insecure URL opening
pub const RULE_ID_URL_OPEN: &str = "CSP-D410";
/// Network: `ssl.wrap_socket` usage
pub const RULE_ID_WRAP_SOCKET: &str = "CSP-D411";

/// Filesystem: Path traversal
pub const RULE_ID_PATH_TRAVERSAL: &str = "CSP-D501";
/// Filesystem: Insecure tarfile extraction
pub const RULE_ID_TARFILE: &str = "CSP-D502";
/// Filesystem: Insecure zipfile extraction
pub const RULE_ID_ZIPFILE: &str = "CSP-D503";
/// Filesystem: Insecure temp file creation
pub const RULE_ID_TEMPFILE: &str = "CSP-D504";
/// Filesystem: Bad file permissions
pub const RULE_ID_PERMISSIONS: &str = "CSP-D505";
/// Filesystem: os.tempnam/os.tmpnam
pub const RULE_ID_TEMPNAM: &str = "CSP-D506";

/// Type Safety: Method misuse
pub const RULE_ID_METHOD_MISUSE: &str = "CSP-D601";

/// Best Practices: Use of assert in production
pub const RULE_ID_ASSERT: &str = "CSP-D701";
/// Best Practices: Insecure module import
pub const RULE_ID_INSECURE_IMPORT: &str = "CSP-D702";
/// Best Practices: Disabled Jinja2 autoescaping
pub const RULE_ID_JINJA_AUTOESCAPE: &str = "CSP-D703";
/// Best Practices: Blacklisted function calls
pub const RULE_ID_BLACKLIST: &str = "CSP-D704";

/// Open Redirect (Taint analysis specific)
pub const RULE_ID_OPEN_REDIRECT: &str = "CSP-D801";

/// Privacy: Logging of sensitive data
pub const RULE_ID_LOGGING_SENSITIVE: &str = "CSP-D901";
/// Privacy: Django `SECRET_KEY` in code
pub const RULE_ID_DJANGO_SECURITY: &str = "CSP-D902";

/// XSS (Generic fallback for taint analysis)
pub const RULE_ID_XSS_GENERIC: &str = "CSP-X001";
