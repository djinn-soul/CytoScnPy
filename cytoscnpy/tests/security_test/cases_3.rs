use super::*;

#[test]
fn test_hardcoded_bind_all_interfaces() {
    let source = r#"
HOST = "0.0.0.0"
IPV6_HOST = "::"
"#;
    scan_danger!(source, linter);
    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.rule_id == "CSP-D404")
        .collect();
    assert_eq!(findings.len(), 2);
}

#[test]
fn test_hardcoded_bind_safe_address() {
    let source = r#"
HOST = "127.0.0.1"
"#;
    scan_danger!(source, linter);
    assert!(!linter.findings.iter().any(|f| f.rule_id == "CSP-D404"));
}

#[test]
fn test_request_without_timeout() {
    let source = r#"
import requests
requests.get("https://example.com")
requests.post("https://example.com", data={})
"#;
    scan_danger!(source, linter);
    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.rule_id == "CSP-D405")
        .collect();
    assert_eq!(findings.len(), 2);
}

#[test]
fn test_request_with_timeout_ok() {
    let source = r#"
import requests
requests.get("https://example.com", timeout=10)
requests.post("https://example.com", timeout=5)
"#;
    scan_danger!(source, linter);
    assert!(!linter.findings.iter().any(|f| f.rule_id == "CSP-D405"));
}

#[test]
fn test_tempfile_mktemp_unsafe() {
    let source = r#"
import tempfile
tempfile.mktemp()
"#;
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D504"));
}

#[test]
fn test_tempfile_mkstemp_safe() {
    let source = r#"
import tempfile
tempfile.mkstemp()
tempfile.TemporaryFile()
"#;
    scan_danger!(source, linter);

    assert!(!linter.findings.iter().any(|f| f.rule_id == "CSP-D504"));
}

// --- NEW RULES TESTS (CSP-D004, D505, D106) ---

#[test]
fn test_insecure_imports() {
    let source = r#"
import telnetlib
import ftplib
import pyghmi
import Crypto.Cipher
from wsgiref.handlers import CGIHandler
"#;
    scan_danger!(source, linter);
    let ids: Vec<_> = linter.findings.iter().map(|f| &f.rule_id).collect();
    // Expect 5 findings
    assert_eq!(linter.findings.len(), 5);
    // Insecure Import is CSP-D702
    assert!(ids.iter().all(|id| *id == "CSP-D702"));
}

#[test]
fn test_bad_file_permissions() {
    let source = r#"
import os
import stat
os.chmod("file", stat.S_IWOTH)
os.chmod("file", mode=stat.S_IWOTH)
"#;
    scan_danger!(source, linter);
    assert_eq!(linter.findings.len(), 2);
    assert!(linter.findings.iter().all(|f| f.rule_id == "CSP-D505"));
}

#[test]
fn test_jinja2_autoescape_false() {
    let source = r#"
import jinja2
env = jinja2.Environment(autoescape=False)
env2 = jinja2.Environment(loader=x, autoescape=False)
"#;
    scan_danger!(source, linter);
    assert_eq!(linter.findings.len(), 2);
    assert!(linter.findings.iter().all(|f| f.rule_id == "CSP-D703"));
}

#[test]
fn test_jinja2_autoescape_true_ok() {
    let source = r#"
import jinja2
env = jinja2.Environment(autoescape=True)
"#;
    scan_danger!(source, linter);
    assert!(!linter.findings.iter().any(|f| f.rule_id == "CSP-D703"));
}

#[test]
fn test_blacklist_calls() {
    let source = r#"
import marshal
import hashlib
import urllib
import telnetlib
import ftplib
import ssl
from django.utils.safestring import mark_safe

marshal.load(f)
hashlib.md5("abc")
urllib.urlopen("http://evil.com")
telnetlib.Telnet("host")
ftplib.FTP("host")
input("prompt")
ssl._create_unverified_context()
mark_safe("<div>")
"#;
    scan_danger!(source, linter);
    let ids: Vec<String> = linter.findings.iter().map(|f| f.rule_id.clone()).collect();

    // Check coverage of B3xx rules
    assert!(ids.contains(&"CSP-D203".to_owned())); // marshal
    assert!(ids.contains(&"CSP-D301".to_owned())); // md5
    assert!(ids.contains(&"CSP-D406".to_owned()) || ids.contains(&"CSP-D410".to_owned())); // urllib
    assert!(ids.contains(&"CSP-D409".to_owned()) || ids.contains(&"CSP-D702".to_owned())); // telnet func
    assert!(ids.contains(&"CSP-D406".to_owned()) || ids.contains(&"CSP-D702".to_owned())); // ftp func
    assert!(ids.contains(&"CSP-D005".to_owned())); // input
    assert!(ids.contains(&"CSP-D408".to_owned())); // ssl (D408 is SSL Unverified, wait context is D408?)
    assert!(ids.contains(&"CSP-D105".to_owned())); // mark_safe
}

#[test]
fn test_pickle_expansion() {
    let source = r#"
import dill
import shelve
import jsonpickle
import pandas

dill.loads(x)
shelve.open("db")
jsonpickle.decode(data)
pandas.read_pickle("file")
"#;
    scan_danger!(source, linter);
    // 6 findings: 2 imports (CSP-D004) + 4 calls (CSP-D201)
    assert_eq!(linter.findings.len(), 6);
    let ids: Vec<String> = linter.findings.iter().map(|f| f.rule_id.clone()).collect();
    assert!(ids.contains(&"CSP-D702".to_owned()));
    assert!(ids.contains(&"CSP-D201".to_owned()));
}

#[test]
fn test_random_rule() {
    let source = r#"
import random
random.random()
random.randint(1, 10)
random.choice([1, 2])
"#;
    scan_danger!(source, linter);
    assert_eq!(linter.findings.len(), 3);
    assert!(linter.findings.iter().all(|f| f.rule_id == "CSP-D311"));
}

#[test]
fn test_xml_extras_and_urllib2() {
    let source = r#"
import xml.dom.pulldom
import xml.dom.expatbuilder
import urllib2
import six

xml.dom.pulldom.parse("file")
xml.dom.expatbuilder.parse("file")
urllib2.urlopen("http://evil.com")
six.moves.urllib.request.urlopen("http://evil.com")
"#;
    scan_danger!(source, linter);
    // 6 findings: 2 imports (CSP-D004) + 4 calls (CSP-D104/D406)
    assert_eq!(linter.findings.len(), 6);

    let ids: Vec<String> = linter.findings.iter().map(|f| f.rule_id.clone()).collect();
    assert!(ids.contains(&"CSP-D104".to_owned())); // pulldom
    assert!(ids.contains(&"CSP-D104".to_owned())); // expatbuilder
    assert!(ids.contains(&"CSP-D410".to_owned())); // urllib2
    assert!(ids.contains(&"CSP-D410".to_owned())); // six.moves
}

#[test]
fn test_b320_lxml_etree() {
    let source = r#"
import lxml.etree as etree
etree.parse("file.xml")
etree.fromstring("<root/>")
etree.RestrictedElement()
etree.GlobalParserTLS()
etree.getDefaultParser()
etree.check_docinfo(doc)
"#;
    scan_danger!(source, linter);
    // 6 calls (CSP-D104) + 1 import (CSP-D004) = 7 findings
    assert_eq!(linter.findings.len(), 7);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D702")); // import
    assert_eq!(
        linter
            .findings
            .iter()
            .filter(|f| f.rule_id == "CSP-D104")
            .count(),
        6
    ); // calls
}

#[test]
fn test_b325_tempnam() {
    let source = r#"
import os
os.tempnam("/tmp", "prefix")
os.tmpnam()
"#;
    scan_danger!(source, linter);
    assert_eq!(linter.findings.len(), 2);
    assert!(linter.findings.iter().all(|f| f.rule_id == "CSP-D506"));
}

#[test]
fn test_blacklist_imports_full_coverage() {
    let source = r#"
import telnetlib
import ftplib
import pickle
import cPickle
import dill
import shelve
import subprocess
import xml.etree.ElementTree
import xml.sax
import xmlrpc
import Crypto.Cipher
from wsgiref.handlers import CGIHandler
import pyghmi
import lxml.etree
"#;
    scan_danger!(source, linter);

    // We expect 14 findings (all imports should be flagged)
    assert_eq!(linter.findings.len(), 14);

    let findings = &linter.findings;

    // Verify high severity
    let high = findings.iter().filter(|f| f.severity == "HIGH").count();
    assert_eq!(high, 6); // telnetlib, ftplib, xmlrpc, Crypto, wsgiref, pyghmi

    // Verify low severity
    let low = findings.iter().filter(|f| f.severity == "LOW").count();
    assert_eq!(low, 8); // pickle, cPickle, dill, shelve, subprocess, xml.etree, xml.sax, lxml

    // Verify rule IDs are generally correct (mostly Imports or Insecure calls)
    assert!(findings.iter().all(|f| f.rule_id == "CSP-D702"
        || f.rule_id == "CSP-D003"
        || f.rule_id == "CSP-D409"
        || f.rule_id == "CSP-D406"));
}
