use cytoscnpy::config::Config;
use cytoscnpy::linter::LinterVisitor;
use cytoscnpy::rules::danger::get_danger_rules;
use cytoscnpy::utils::LineIndex;
use ruff_python_parser::{parse, Mode};
use std::path::PathBuf;

macro_rules! scan_danger {
    ($source:expr, $linter:ident) => {
        let tree = parse($source, Mode::Module.into()).expect("Failed to parse");
        let line_index = LineIndex::new($source);
        let rules = get_danger_rules();
        let config = Config::default();
        let mut $linter = LinterVisitor::new(rules, PathBuf::from("test.py"), line_index, config);

        if let ruff_python_ast::Mod::Module(module) = tree.into_syntax() {
            for stmt in &module.body {
                $linter.visit_stmt(stmt);
            }
        }
    };
}

#[test]
fn test_extensive_security_corpus() {
    let mut corpus_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    corpus_path.push("tests/python_files/extensive_security_corpus.py");
    let source = std::fs::read_to_string(&corpus_path).expect("Failed to read corpus file");

    scan_danger!(&source, linter);

    let findings = &linter.findings;

    // Total findings count
    println!("Total findings: {}", findings.len());

    // Rule ID check
    let ids: std::collections::HashSet<_> = findings.iter().map(|f| f.rule_id.as_str()).collect();

    // Execution
    assert!(ids.contains("CSP-D001"), "Missing CSP-D001 (eval)");
    assert!(ids.contains("CSP-D002"), "Missing CSP-D002 (exec)");
    assert!(ids.contains("CSP-D003"), "Missing CSP-D003 (os.system)");

    // Network/Bind
    assert!(
        ids.contains("CSP-D404"),
        "Missing CSP-D404 (Hardcoded Bind)"
    );
    assert!(
        ids.contains("CSP-D405"),
        "Missing CSP-D405 (Request Timeout)"
    );
    assert!(
        ids.contains("CSP-D407"),
        "Missing CSP-D407 (Unverified SSL)"
    );
    assert!(
        ids.contains("CSP-D408"),
        "Missing CSP-D408 (HTTPS Connection)"
    );

    // Crypto/Hashes
    assert!(ids.contains("CSP-D301"), "Missing CSP-D301 (MD5)");
    assert!(ids.contains("CSP-D302"), "Missing CSP-D302 (SHA1)");
    assert!(
        ids.contains("CSP-D304"),
        "Missing CSP-D304 (Insecure Cipher)"
    );
    assert!(ids.contains("CSP-D305"), "Missing CSP-D305 (Insecure Mode)");
    assert!(ids.contains("CSP-D311"), "Missing CSP-D311 (Random)");

    // Injection/XML
    assert!(ids.contains("CSP-D104"), "Missing CSP-D104 (XML)");
    assert!(ids.contains("CSP-D105"), "Missing CSP-D105 (Assert)");
    assert!(ids.contains("CSP-D106"), "Missing CSP-D106 (Jinja2)");

    // Deserialization
    assert!(ids.contains("CSP-D201"), "Missing CSP-D201 (Pickle)");
    assert!(ids.contains("CSP-D203"), "Missing CSP-D203 (Marshal)");

    // Files/Temp
    assert!(ids.contains("CSP-D504"), "Missing CSP-D504 (mktemp)");
    assert!(ids.contains("CSP-D505"), "Missing CSP-D505 (chmod)");
    assert!(ids.contains("CSP-D506"), "Missing CSP-D506 (tempnam)");

    // Misc
    assert!(ids.contains("CSP-D403"), "Missing CSP-D403 (Debug)");
    assert!(ids.contains("CSP-D402"), "Missing CSP-D402 (SSRF)");
}
