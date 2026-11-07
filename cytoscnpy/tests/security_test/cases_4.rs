use super::*;

#[test]
fn test_b324_hashlib_new_unsafe() {
    let source = r#"
import hashlib
hashlib.new("md5", b"data")
hashlib.new("sha1", b"data")
hashlib.new("MD5", b"data")
hashlib.new("sha256", b"data") # secure
"#;
    scan_danger!(source, linter);
    assert_eq!(linter.findings.len(), 3);
    // MD5 should be CSP-D301, SHA1 should be CSP-D302
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D301"));
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D302"));
}

#[test]
fn test_b309_https_connection_unsafe() {
    let source = r#"
import http.client
import ssl
http.client.HTTPSConnection("host")
http.client.HTTPSConnection("host", context=ssl.create_default_context()) # safe
"#;
    scan_danger!(source, linter);
    assert_eq!(linter.findings.len(), 1);
    assert!(linter.findings.iter().all(|f| f.rule_id == "CSP-D407"));
}

#[test]
fn test_request_timeout_refined() {
    let source = r#"
import requests
import httpx
requests.get("url", timeout=None) # unsafe
requests.post("url", timeout=0)    # unsafe
httpx.post("url", timeout=False)  # unsafe
httpx.get("url", timeout=5)       # safe
requests.get("url", timeout=5.0)  # safe
"#;
    scan_danger!(source, linter);
    assert_eq!(linter.findings.len(), 3);
    assert!(linter.findings.iter().all(|f| f.rule_id == "CSP-D405"));
}

#[test]
fn test_request_timeout_case_sensitive_method_match() {
    let source = r#"
import requests
requests.Get("url")
requests.get("url")
"#;
    scan_danger!(source, linter);
    let timeout_findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.rule_id == "CSP-D405")
        .collect();

    assert_eq!(
        timeout_findings.len(),
        1,
        "Only lowercase requests.get should match timeout rule"
    );
}

#[test]
fn test_socket_bind_positional() {
    let source = r#"
import socket
s = socket.socket()
s.bind(("0.0.0.0", 80)) # unsafe
s.bind(("127.0.0.1", 80)) # safe
"#;
    scan_danger!(source, linter);
    assert_eq!(linter.findings.len(), 1);
    assert_eq!(linter.findings[0].rule_id, "CSP-D404");
}

// -------------------------------------------------------------------------
// CSP-D412: MCP StdioServerParameters non-literal command
// -------------------------------------------------------------------------

#[test]
fn test_mcp_stdio_non_literal_command_kwarg() {
    let source = r#"
from mcp import StdioServerParameters
cmd = get_user_command()
params = StdioServerParameters(command=cmd, args=[])
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D412"),
        "Non-literal command kwarg should trigger CSP-D412"
    );
}

#[test]
fn test_mcp_stdio_literal_command_safe() {
    let source = r#"
from mcp import StdioServerParameters
params = StdioServerParameters(command="uvx", args=["my-server"])
"#;
    scan_danger!(source, linter);
    assert!(
        !linter.findings.iter().any(|f| f.rule_id == "CSP-D412"),
        "Literal string command should not trigger CSP-D412"
    );
}

#[test]
fn test_mcp_stdio_fstring_command_flagged() {
    let source = r#"
from mcp import StdioServerParameters
server = user_input
params = StdioServerParameters(command=f"run-{server}", args=[])
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D412"),
        "F-string command should trigger CSP-D412"
    );
}

#[test]
fn test_mcp_stdio_non_literal_positional_flagged() {
    let source = r#"
from mcp import StdioServerParameters
cmd = get_cmd()
params = StdioServerParameters(cmd, args=[])
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D412"),
        "Non-literal positional command should trigger CSP-D412"
    );
}

// -------------------------------------------------------------------------
// CSP-D903: Django @csrf_exempt decorator
// -------------------------------------------------------------------------

#[test]
fn test_csrf_exempt_on_function() {
    let source = r#"
from django.views.decorators.csrf import csrf_exempt

@csrf_exempt
def my_view(request):
    return HttpResponse("ok")
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D903"),
        "@csrf_exempt on function should trigger CSP-D903"
    );
}

#[test]
fn test_csrf_exempt_qualified_name() {
    let source = r#"
import django.views.decorators.csrf

@django.views.decorators.csrf.csrf_exempt
def my_view(request):
    return HttpResponse("ok")
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D903"),
        "Qualified @csrf_exempt should trigger CSP-D903"
    );
}

#[test]
fn test_no_csrf_exempt_no_finding() {
    let source = r#"
from django.views.decorators.csrf import csrf_protect

@csrf_protect
def my_view(request):
    return HttpResponse("ok")
"#;
    scan_danger!(source, linter);
    assert!(
        !linter.findings.iter().any(|f| f.rule_id == "CSP-D903"),
        "@csrf_protect should not trigger CSP-D903"
    );
}

// -------------------------------------------------------------------------
// CSP-D106: LDAP Injection
// -------------------------------------------------------------------------

#[test]
fn test_ldap_injection_search_s_positional() {
    let source = r#"
import ldap
user_input = request.args.get("user")
ldap_filter = f"(uid={user_input})"
conn = ldap.initialize("ldap://example.com")
conn.search_s("dc=example,dc=com", ldap.SCOPE_SUBTREE, ldap_filter)
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D106"),
        "Non-literal LDAP filter positional arg should trigger CSP-D106"
    );
}

#[test]
fn test_ldap_injection_filterstr_kwarg() {
    let source = r#"
import ldap
user = get_user()
ldap_filter = "(uid=" + user + ")"
ldap.search_s("dc=example,dc=com", ldap.SCOPE_SUBTREE, filterstr=ldap_filter)
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D106"),
        "Non-literal filterstr kwarg should trigger CSP-D106"
    );
}

#[test]
fn test_ldap_injection_literal_filter_safe() {
    let source = r#"
import ldap
conn.search_s("dc=example,dc=com", ldap.SCOPE_SUBTREE, "(objectClass=person)")
"#;
    scan_danger!(source, linter);
    assert!(
        !linter.findings.iter().any(|f| f.rule_id == "CSP-D106"),
        "Literal LDAP filter should not trigger CSP-D106"
    );
}

#[test]
fn test_ldap3_search_ext_s_flagged() {
    // ldap3 also exposes search_ext_s which IS in the known list
    let source = r#"
import ldap
user = request.form["username"]
ldap_filter = f"(uid={user})"
conn = ldap.initialize("ldap://example.com")
conn.search_ext_s("dc=example,dc=com", ldap.SCOPE_SUBTREE, ldap_filter)
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D106"),
        "search_ext_s with non-literal filter should trigger CSP-D106"
    );
}

#[test]
fn test_ldap3_connection_search_filter_kwarg() {
    let source = r#"
from ldap3 import Connection
user = request.form["username"]
conn = Connection(server)
conn.search("dc=example,dc=com", search_filter=f"(uid={user})")
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D106"),
        "Non-literal ldap3 Connection.search search_filter kwarg should trigger CSP-D106"
    );
}

#[test]
fn test_ldap_non_ldap_search_filter_kwarg_no_trigger() {
    // A non-ldap function with a search_filter kwarg must not fire CSP-D106
    let source = r#"
results = engine.search(query, search_filter=user_filter)
"#;
    scan_danger!(source, linter);
    assert!(
        !linter.findings.iter().any(|f| f.rule_id == "CSP-D106"),
        "generic engine.search(search_filter=) without ldap context should not trigger CSP-D106"
    );
}

#[test]
fn test_ldap_non_ldap_search_s_no_trigger() {
    let source = r#"
result = index.search_s(query, filterstr=user_filter)
"#;
    scan_danger!(source, linter);
    assert!(
        !linter.findings.iter().any(|f| f.rule_id == "CSP-D106"),
        "generic search_s(filterstr=) without ldap context should not trigger CSP-D106"
    );
}
