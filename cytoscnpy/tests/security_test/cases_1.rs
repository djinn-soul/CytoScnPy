use super::*;

#[test]
fn test_eval_detection() {
    let source = r#"
user_input = input("Enter code: ")
result = eval(user_input)
"#;
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D001"));
}

#[test]
fn test_exec_detection() {
    let source = r#"
code = "print('hello')"
exec(code)
"#;
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D002"));
}

#[test]
fn test_pickle_loads() {
    let source = "import pickle\npickle.loads(b'\\x80\\x04K\\x01.')\n";
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D201"));
}

#[test]
fn test_yaml_load_without_safeloader() {
    let source = "import yaml\nyaml.load('a: 1')\n";
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D202"));
}

#[test]
fn test_md5_sha1() {
    let source = "import hashlib\nhashlib.md5(b'd')\nhashlib.sha1(b'd')\n";
    scan_danger!(source, linter);
    let ids: Vec<_> = linter.findings.iter().map(|f| &f.rule_id).collect();
    assert!(ids.contains(&&"CSP-D301".to_owned()));
    assert!(ids.contains(&&"CSP-D302".to_owned()));
}

#[test]
fn test_subprocess_shell_true() {
    let source = "import subprocess\ncmd = 'echo hi'\nsubprocess.run(cmd, shell=True)\n";
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D003"));
}

#[test]
fn test_requests_verify_false() {
    let source = "import requests\nrequests.get('https://x', verify=False)\n";
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D401"));
}

#[test]
fn test_yaml_safe_loader_does_not_trigger() {
    let source = "import yaml\nfrom yaml import SafeLoader\nyaml.load('a: 1', Loader=SafeLoader)\n";
    scan_danger!(source, linter);
    assert!(!linter.findings.iter().any(|f| f.rule_id == "CSP-D202"));
}

#[test]
fn test_tarfile_extractall_unsafe() {
    let source = r#"
import tarfile
tar = tarfile.open("archive.tar")
tar.extractall()
"#;
    scan_danger!(source, linter);
    let finding = linter.findings.iter().find(|f| f.rule_id == "CSP-D502");
    assert!(finding.is_some());
    assert_eq!(finding.unwrap().severity, "HIGH");
}

#[test]
fn test_tarfile_extractall_chained_call() {
    // Chained call: tarfile.open(...).extractall()
    let source = r#"
import tarfile
tarfile.open("archive.tar").extractall()
"#;
    scan_danger!(source, linter);
    let finding = linter.findings.iter().find(|f| f.rule_id == "CSP-D502");
    assert!(finding.is_some());
    assert_eq!(finding.unwrap().severity, "HIGH");
}

#[test]
fn test_tarfile_extractall_with_filter_data_is_safe() {
    let source = r#"
import tarfile
tar = tarfile.open("archive.tar")
tar.extractall(filter='data')
"#;
    scan_danger!(source, linter);
    assert!(!linter.findings.iter().any(|f| f.rule_id == "CSP-D502"));
}

#[test]
fn test_tarfile_extractall_with_filter_tar_is_safe() {
    let source = r#"
import tarfile
tarfile.open("archive.tar").extractall(filter='tar')
"#;
    scan_danger!(source, linter);
    assert!(!linter.findings.iter().any(|f| f.rule_id == "CSP-D502"));
}

#[test]
fn test_tarfile_extractall_with_nonliteral_filter() {
    // Non-literal filter should be flagged with MEDIUM severity
    let source = r#"
import tarfile
tar = tarfile.open("archive.tar")
tar.extractall(filter=my_custom_filter)
"#;
    scan_danger!(source, linter);
    let finding = linter.findings.iter().find(|f| f.rule_id == "CSP-D502");
    assert!(finding.is_some());
    assert_eq!(finding.unwrap().severity, "MEDIUM");
}

#[test]
fn test_tarfile_extractall_with_path_but_no_filter() {
    let source = r#"
import tarfile
tar = tarfile.open("archive.tar")
tar.extractall(path="/tmp")
"#;
    scan_danger!(source, linter);
    let finding = linter.findings.iter().find(|f| f.rule_id == "CSP-D502");
    assert!(finding.is_some());
    assert_eq!(finding.unwrap().severity, "HIGH");
}

#[test]
fn test_unrelated_extractall_lower_severity() {
    // Random object with extractall() should be MEDIUM (not HIGH)
    let source = r#"
some_object.extractall()
"#;
    scan_danger!(source, linter);
    let finding = linter.findings.iter().find(|f| f.rule_id == "CSP-D502");
    assert!(finding.is_some());
    assert_eq!(finding.unwrap().severity, "MEDIUM");
}

// --- ZIPFILE TESTS (CSP-D503) ---

#[test]
fn test_zipfile_extractall_unsafe() {
    let source = r#"
import zipfile
zip = zipfile.ZipFile("archive.zip")
zip.extractall()
"#;
    scan_danger!(source, linter);
    let finding = linter.findings.iter().find(|f| f.rule_id == "CSP-D503");
    assert!(finding.is_some());
    assert_eq!(finding.unwrap().severity, "HIGH");
}

#[test]
fn test_zipfile_extractall_chained() {
    let source = r#"
import zipfile
zipfile.ZipFile("archive.zip").extractall()
"#;
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D503"));
}

#[test]
fn test_zipfile_extractall_variable_named_zip() {
    let source = r#"
zip_archive.extractall()
"#;
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D503"));
}

#[test]
fn test_unrelated_extractall_no_zipfile_finding() {
    // Random object that doesn't look like zipfile
    let source = r#"
my_data.extractall()
"#;
    scan_danger!(source, linter);
    // Should NOT trigger CSP-D503 (zipfile rule)
    assert!(!linter.findings.iter().any(|f| f.rule_id == "CSP-D503"));
}

// --- XML PARSING TESTS (CSP-D104) ---

#[test]
fn test_xml_etree_parse_unsafe() {
    let source = r#"
import xml.etree.ElementTree as ET
ET.parse("file.xml")
"#;
    scan_danger!(source, linter);
    let finding = linter.findings.iter().find(|f| f.rule_id == "CSP-D104");
    assert!(finding.is_some());
    assert_eq!(finding.unwrap().severity, "MEDIUM");
}

#[test]
fn test_xml_etree_fromstring_unsafe() {
    let source = r#"
import xml.etree.ElementTree as ET
ET.fromstring("<root>...</root>")
"#;
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D104"));
}

#[test]
fn test_xml_dom_minidom_parse_unsafe() {
    let source = r#"
import xml.dom.minidom
xml.dom.minidom.parse("file.xml")
"#;
    scan_danger!(source, linter);
    let finding = linter.findings.iter().find(|f| f.rule_id == "CSP-D104");
    assert!(finding.is_some());
    assert!(finding.unwrap().message.contains("minidom"));
}

#[test]
fn test_xml_sax_parse_unsafe() {
    let source = r#"
import xml.sax
xml.sax.parse("file.xml", None)
"#;
    scan_danger!(source, linter);
    let finding = linter.findings.iter().find(|f| f.rule_id == "CSP-D104");
    assert!(finding.is_some());
    assert!(finding.unwrap().message.contains("XXE"));
}

#[test]
fn test_xml_sax_make_parser_unsafe() {
    let source = r#"
import xml.sax
xml.sax.make_parser()
"#;
    scan_danger!(source, linter);
    assert!(linter.findings.iter().any(|f| f.rule_id == "CSP-D104"));
}

#[test]
fn test_lxml_etree_parse_high_severity() {
    let source = r#"
import lxml.etree
lxml.etree.parse("file.xml")
"#;
    scan_danger!(source, linter);
    let finding = linter.findings.iter().find(|f| f.rule_id == "CSP-D104");
    assert!(finding.is_some());
    assert_eq!(finding.unwrap().severity, "HIGH");
}

#[test]
fn test_lxml_etree_fromstring_high_severity() {
    let source = r#"
import lxml.etree
lxml.etree.fromstring("<root>...</root>")
"#;
    scan_danger!(source, linter);
    let finding = linter.findings.iter().find(|f| f.rule_id == "CSP-D104");
    assert!(finding.is_some());
    assert_eq!(finding.unwrap().severity, "HIGH");
    assert!(finding.unwrap().message.contains("XXE"));
}

#[test]
fn test_defusedxml_is_safe() {
    // defusedxml with a different alias should NOT trigger findings
    // Note: ET.parse is flagged because we can't track import aliases statically
    // Using a different alias name demonstrates safe xml parsing
    let source = r#"
import defusedxml.ElementTree as SafeET
SafeET.parse("file.xml")
SafeET.fromstring("<root>...</root>")
"#;
    scan_danger!(source, linter);
    assert!(!linter.findings.iter().any(|f| f.rule_id == "CSP-D104"));
}

#[test]
fn test_subprocess_without_shell_true_is_ok() {
    let source = "import subprocess\nsubprocess.run(['echo','hi'])\n";
    scan_danger!(source, linter);
    assert!(!linter.findings.iter().any(|f| f.rule_id == "CSP-D003"));
}
