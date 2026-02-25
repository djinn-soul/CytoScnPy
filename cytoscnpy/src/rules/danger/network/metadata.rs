use crate::rules::ids;
use crate::rules::RuleMetadata;

/// Metadata for the insecure requests verification rule.
pub const META_REQUESTS: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_REQUESTS,
    category: super::super::CAT_NETWORK,
};
/// Metadata for the SSRF rule.
pub const META_SSRF: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_SSRF,
    category: super::super::CAT_NETWORK,
};
/// Metadata for the debug mode exposure rule.
pub const META_DEBUG_MODE: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_DEBUG_MODE,
    category: super::super::CAT_NETWORK,
};
/// Metadata for the bind-all-interfaces rule.
pub const META_BIND_ALL: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_BIND_ALL,
    category: super::super::CAT_NETWORK,
};
/// Metadata for the missing-timeout request rule.
pub const META_TIMEOUT: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_TIMEOUT,
    category: super::super::CAT_NETWORK,
};
/// Metadata for the insecure FTP usage rule.
pub const META_FTP: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_FTP,
    category: super::super::CAT_NETWORK,
};
/// Metadata for insecure `HTTPSConnection` usage rule.
pub const META_HTTPS_CONNECTION: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_HTTPS_CONNECTION,
    category: super::super::CAT_NETWORK,
};
/// Metadata for unverified SSL context usage rule.
pub const META_SSL_UNVERIFIED: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_SSL_UNVERIFIED,
    category: super::super::CAT_NETWORK,
};
/// Metadata for insecure Telnet usage rule.
pub const META_TELNET: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_TELNET,
    category: super::super::CAT_NETWORK,
};
/// Metadata for unsafe URL-open usage rule.
pub const META_URL_OPEN: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_URL_OPEN,
    category: super::super::CAT_NETWORK,
};
/// Metadata for deprecated `ssl.wrap_socket` usage rule.
pub const META_WRAP_SOCKET: RuleMetadata = RuleMetadata {
    id: ids::RULE_ID_WRAP_SOCKET,
    category: super::super::CAT_NETWORK,
};
