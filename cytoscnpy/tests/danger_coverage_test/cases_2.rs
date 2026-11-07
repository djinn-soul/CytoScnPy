use super::*;

#[test]
fn test_jinjasql_instance_taint_detection() {
    let source = "
from jinjasql import JinjaSql
j = JinjaSql()
user_sql = input()
j.prepare_query(user_sql, {})
";
    let mut config = cytoscnpy::config::Config::default();
    config.cytoscnpy.danger_config.enable_taint = Some(true);

    let analyzer = CytoScnPy::default().with_danger(true).with_config(config);

    let result = analyzer.analyze_code(source, &PathBuf::from("test.py"));

    let sqli_findings: Vec<_> = result
        .danger
        .iter()
        .filter(|f| f.rule_id == "CSP-D102")
        .collect();

    assert!(
        !sqli_findings.is_empty(),
        "j.prepare_query(user_sql, {{}}) should be flagged"
    );
    assert_eq!(sqli_findings[0].severity, "CRITICAL");
}

#[test]
fn test_os_path_join_all_args_tainted() {
    let source = "
import os
tainted = input()
os.path.join('a', 'b', 'c', tainted)
";
    let mut config = cytoscnpy::config::Config::default();
    config.cytoscnpy.danger_config.enable_taint = Some(true);

    let analyzer = CytoScnPy::default().with_danger(true).with_config(config);

    let result = analyzer.analyze_code(source, &PathBuf::from("test.py"));

    let path_findings: Vec<_> = result
        .danger
        .iter()
        .filter(|f| f.rule_id == "CSP-D501")
        .collect();

    assert!(
        !path_findings.is_empty(),
        "os.path.join('a', 'b', 'c', tainted) should be flagged"
    );
}
