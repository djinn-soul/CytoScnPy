use super::*;

// -------------------------------------------------------------------------
// CSP-D107: XPath Injection
// -------------------------------------------------------------------------

#[test]
fn test_xpath_injection_dynamic_expr() {
    let source = r#"
from lxml import etree
user_input = request.args.get("xpath")
tree = etree.parse("data.xml")
result = tree.xpath(user_input)
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D107"),
        "Dynamic xpath() arg should trigger CSP-D107"
    );
}

#[test]
fn test_xpath_injection_lxml_xpath_constructor() {
    let source = r#"
from lxml import etree
user_filter = request.args.get("filter")
expr = etree.XPath(user_filter)
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D107"),
        "Dynamic lxml.etree.XPath() arg should trigger CSP-D107"
    );
}

#[test]
fn test_xpath_injection_literal_safe() {
    let source = r#"
from lxml import etree
tree = etree.parse("data.xml")
result = tree.xpath("//user[@name='admin']")
"#;
    scan_danger!(source, linter);
    assert!(
        !linter.findings.iter().any(|f| f.rule_id == "CSP-D107"),
        "Literal xpath expression should not trigger CSP-D107"
    );
}

#[test]
fn test_xpath_injection_fstring_expr() {
    let source = r#"
from lxml import etree
username = get_user()
tree = etree.parse("data.xml")
result = tree.xpath(f"//user[@name='{username}']")
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D107"),
        "F-string xpath expression should trigger CSP-D107"
    );
}

// -------------------------------------------------------------------------
// CSP-D006: Privilege Escalation
// -------------------------------------------------------------------------

#[test]
fn test_priv_escalation_setuid() {
    let source = r#"
import os
os.setuid(0)
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D006"),
        "os.setuid(0) should trigger CSP-D006"
    );
}

#[test]
fn test_priv_escalation_setgid() {
    let source = r#"
import os
os.setgid(0)
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D006"),
        "os.setgid(0) should trigger CSP-D006"
    );
}

#[test]
fn test_priv_escalation_setreuid() {
    let source = r#"
import os
os.setreuid(0, 0)
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D006"),
        "os.setreuid(0, 0) should trigger CSP-D006"
    );
}

#[test]
fn test_priv_escalation_bare_attr() {
    let source = r#"
uid = get_uid()
proc.setuid(uid)
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D006"),
        "bare .setuid() attr should trigger CSP-D006"
    );
}

// -------------------------------------------------------------------------
// CSP-D904: Log Injection
// -------------------------------------------------------------------------

#[test]
fn test_log_injection_fstring() {
    let source = r#"
import logging
user_input = request.args.get("name")
logging.info(f"User logged in: {user_input}")
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D904"),
        "F-string log arg should trigger CSP-D904"
    );
}

#[test]
fn test_log_injection_string_concat() {
    let source = r#"
import logging
username = get_username()
logging.warning("Login attempt: " + username)
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D904"),
        "String concat log arg should trigger CSP-D904"
    );
}

#[test]
fn test_log_injection_percent_format() {
    let source = r#"
import logging
user = get_user()
logging.error("Access denied for %s" % user)
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D904"),
        "Percent-format log arg should trigger CSP-D904"
    );
}

#[test]
fn test_log_injection_literal_safe() {
    let source = r#"
import logging
logging.info("Application started")
"#;
    scan_danger!(source, linter);
    assert!(
        !linter.findings.iter().any(|f| f.rule_id == "CSP-D904"),
        "Literal string log arg should not trigger CSP-D904"
    );
}

#[test]
fn test_log_injection_literal_fstring_no_trigger() {
    // Regression: an f-string with zero interpolations (e.g. f"started") is a
    // pure literal and must not be flagged as injectable.
    let source = r#"
import logging
logger = logging.getLogger(__name__)
logger.info(f"started")
"#;
    scan_danger!(source, linter);
    assert!(
        !linter.findings.iter().any(|f| f.rule_id == "CSP-D904"),
        "Literal-only f-string log arg should not trigger CSP-D904"
    );
}

#[test]
fn test_ldap3_connection_search_positional_filter_flagged() {
    // Regression: ldap3 Connection.search puts the filter at positional index
    // 1 (search_base, search_filter, ...), not index 2 like python-ldap.
    let source = r#"
from ldap3 import Connection
user = request.form["username"]
conn = Connection(server)
conn.search("dc=example,dc=com", f"(uid={user})")
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D106"),
        "ldap3 Connection.search positional filter should trigger CSP-D106"
    );
}

#[test]
fn test_ldap3_connection_search_literal_positional_safe() {
    let source = r#"
from ldap3 import Connection
conn = Connection(server)
conn.search("dc=example,dc=com", "(objectClass=person)")
"#;
    scan_danger!(source, linter);
    assert!(
        !linter.findings.iter().any(|f| f.rule_id == "CSP-D106"),
        "ldap3 Connection.search literal filter must not trigger CSP-D106"
    );
}

#[test]
fn test_python_ldap_connection_search_filter_index_flagged() {
    let source = r#"
import ldap
user = request.form["username"]
conn = ldap.initialize("ldap://example.com")
conn.search("dc=example,dc=com", ldap.SCOPE_SUBTREE, f"(uid={user})")
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D106"),
        "python-ldap conn.search positional filter at index 2 should trigger CSP-D106"
    );
}

#[test]
fn test_python_ldap_connection_search_literal_filter_safe() {
    let source = r#"
import ldap
conn = ldap.initialize("ldap://example.com")
conn.search("dc=example,dc=com", ldap.SCOPE_SUBTREE, "(objectClass=person)")
"#;
    scan_danger!(source, linter);
    assert!(
        !linter.findings.iter().any(|f| f.rule_id == "CSP-D106"),
        "python-ldap conn.search literal filter should not treat scope as injectable"
    );
}

// -------------------------------------------------------------------------
// CSP-D705: Hardcoded Default Credentials
// -------------------------------------------------------------------------

#[test]
fn test_hardcoded_creds_user_admin() {
    let source = r#"
if username == "admin":
    grant_access()
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D705"),
        "username == 'admin' should trigger CSP-D705"
    );
}

#[test]
fn test_hardcoded_creds_password_eq() {
    let source = r#"
if password == "password":
    login()
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D705"),
        "password == 'password' should trigger CSP-D705"
    );
}

#[test]
fn test_hardcoded_creds_reversed() {
    let source = r#"
if "admin" == user:
    allow()
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D705"),
        "'admin' == user (reversed) should trigger CSP-D705"
    );
}
