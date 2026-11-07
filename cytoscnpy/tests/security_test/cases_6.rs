use super::*;

#[test]
fn test_hardcoded_creds_non_default_no_trigger() {
    let source = r#"
if username == stored_username:
    login()
"#;
    scan_danger!(source, linter);
    assert!(
        !linter.findings.iter().any(|f| f.rule_id == "CSP-D705"),
        "variable comparison should not trigger CSP-D705"
    );
}

#[test]
fn test_hardcoded_creds_empty_string_no_trigger() {
    let source = r#"
if password == "":
    raise ValueError("password cannot be empty")
"#;
    scan_danger!(source, linter);
    assert!(
        !linter.findings.iter().any(|f| f.rule_id == "CSP-D705"),
        "defensive empty-string check should not trigger CSP-D705"
    );
}

// -------------------------------------------------------------------------
// CSP-D507: TOCTOU Race Condition
// -------------------------------------------------------------------------

#[test]
fn test_race_condition_exists_then_open() {
    let source = r#"
import os
path = get_user_path()
if os.path.exists(path):
    with open(path) as f:
        data = f.read()
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D507"),
        "os.path.exists() + open() should trigger CSP-D507"
    );
}

#[test]
fn test_race_condition_isfile_then_open() {
    let source = r#"
import os
if os.path.isfile(filename):
    f = open(filename)
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D507"),
        "os.path.isfile() + open() should trigger CSP-D507"
    );
}

#[test]
fn test_race_condition_not_exists_no_open_safe() {
    let source = r#"
import os
if not os.path.exists(path):
    raise FileNotFoundError("not found")
"#;
    scan_danger!(source, linter);
    assert!(
        !linter.findings.iter().any(|f| f.rule_id == "CSP-D507"),
        "exists() without open() in body should not trigger CSP-D507"
    );
}

#[test]
fn test_race_condition_access_then_open() {
    let source = r#"
import os
if os.access(path, os.R_OK):
    content = open(path).read()
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D507"),
        "os.access() + open() should trigger CSP-D507"
    );
}

#[test]
fn test_race_condition_else_fallback_open_no_trigger() {
    let source = r#"
import os
if os.path.exists(path):
    use_cached_metadata(path)
else:
    with open("fallback.txt") as f:
        data = f.read()
"#;
    scan_danger!(source, linter);
    assert!(
        !linter.findings.iter().any(|f| f.rule_id == "CSP-D507"),
        "fallback open in else branch should not trigger CSP-D507 for checked path"
    );
}

#[test]
fn test_race_condition_unrelated_open_path_no_trigger() {
    let source = r#"
import os
if os.path.exists(path):
    with open(other_path) as f:
        data = f.read()
"#;
    scan_danger!(source, linter);
    assert!(
        !linter.findings.iter().any(|f| f.rule_id == "CSP-D507"),
        "open() of a different path should not trigger CSP-D507"
    );
}

#[test]
fn test_race_condition_generic_open_method_no_trigger() {
    let source = r#"
import os
if os.path.exists(path):
    session.open(path)
"#;
    scan_danger!(source, linter);
    assert!(
        !linter.findings.iter().any(|f| f.rule_id == "CSP-D507"),
        "generic .open() methods should not trigger CSP-D507"
    );
}

#[test]
fn test_race_condition_pathlib_instance_exists_then_open() {
    let source = r#"
from pathlib import Path
p = Path(name)
if p.exists():
    with p.open() as f:
        data = f.read()
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D507"),
        "Path.exists() followed by Path.open() should trigger CSP-D507"
    );
}

#[test]
fn test_race_condition_pathlib_instance_is_file_then_builtin_open() {
    let source = r#"
from pathlib import Path
p = Path(name)
if p.is_file():
    with open(p) as f:
        data = f.read()
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D507"),
        "Path.is_file() followed by open(path_object) should trigger CSP-D507"
    );
}

// -------------------------------------------------------------------------
// CSP-D306: PyNaCl Low-level Bindings
// -------------------------------------------------------------------------

#[test]
fn test_pynacl_lowlevel_import() {
    let source = r#"
import nacl.bindings
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D306"),
        "import nacl.bindings should trigger CSP-D306"
    );
}

#[test]
fn test_pynacl_lowlevel_from_import() {
    let source = r#"
from nacl.bindings import crypto_secretbox
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D306"),
        "from nacl.bindings import should trigger CSP-D306"
    );
}

#[test]
fn test_pynacl_lowlevel_call() {
    let source = r#"
from nacl.bindings import crypto_secretbox
ct = nacl.bindings.crypto_secretbox(msg, nonce, key)
"#;
    scan_danger!(source, linter);
    assert!(
        linter.findings.iter().any(|f| f.rule_id == "CSP-D306"),
        "nacl.bindings.* call should trigger CSP-D306"
    );
}

#[test]
fn test_pynacl_highlevel_safe() {
    let source = r#"
from nacl.secret import SecretBox
box = SecretBox(key)
"#;
    scan_danger!(source, linter);
    assert!(
        !linter.findings.iter().any(|f| f.rule_id == "CSP-D306"),
        "nacl.secret.SecretBox should not trigger CSP-D306"
    );
}

#[test]
fn test_pynacl_user_defined_crypto_prefix_no_trigger() {
    // User-defined functions with crypto_ prefix must not fire CSP-D306
    let source = r#"
def crypto_box_open(ciphertext, key):
    return decrypt(ciphertext, key)

result = crypto_box_open(data, my_key)
"#;
    scan_danger!(source, linter);
    assert!(
        !linter.findings.iter().any(|f| f.rule_id == "CSP-D306"),
        "user-defined crypto_box_open should not trigger CSP-D306"
    );
}

// -------------------------------------------------------------------------
// CSP-D904: Log injection FP — non-logger .error/.info/.debug methods
// -------------------------------------------------------------------------

#[test]
fn test_log_injection_result_error_no_trigger() {
    let source = r#"
result.error(f"failed: {reason}")
"#;
    scan_danger!(source, linter);
    assert!(
        !linter.findings.iter().any(|f| f.rule_id == "CSP-D904"),
        "result.error() should not trigger CSP-D904"
    );
}

#[test]
fn test_log_injection_click_echo_no_trigger() {
    let source = r#"
import click
click.echo(f"Hello {name}")
"#;
    scan_danger!(source, linter);
    assert!(
        !linter.findings.iter().any(|f| f.rule_id == "CSP-D904"),
        "click.echo() should not trigger CSP-D904"
    );
}

// -------------------------------------------------------------------------
// CSP-D507: TOCTOU FP — ORM queryset.exists() should not trigger
// -------------------------------------------------------------------------

#[test]
fn test_race_condition_orm_exists_no_trigger() {
    let source = r#"
if User.objects.filter(email=email).exists():
    with open("report.txt") as f:
        data = f.read()
"#;
    scan_danger!(source, linter);
    assert!(
        !linter.findings.iter().any(|f| f.rule_id == "CSP-D507"),
        "ORM queryset.exists() should not trigger CSP-D507"
    );
}

#[test]
fn test_race_condition_redis_exists_no_trigger() {
    let source = r#"
import redis
r = redis.Redis()
if r.exists("my_key"):
    with open("cache.txt") as f:
        data = f.read()
"#;
    scan_danger!(source, linter);
    assert!(
        !linter.findings.iter().any(|f| f.rule_id == "CSP-D507"),
        "redis.exists() should not trigger CSP-D507"
    );
}
