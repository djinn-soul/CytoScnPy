use super::{Severity, SinkInfo, VulnType};
use crate::rules::ids;

pub(super) fn check_code_injection_sinks(name: &str) -> Option<SinkInfo> {
    match name {
        "eval" => Some(SinkInfo {
            name: "eval".to_owned(),
            rule_id: ids::RULE_ID_EVAL.to_owned(),
            vuln_type: VulnType::CodeInjection,
            severity: Severity::Critical,
            dangerous_args: vec![0],
            dangerous_keywords: Vec::new(),
            remediation: "Avoid eval() with user input. Use ast.literal_eval() for safe parsing."
                .to_owned(),
        }),
        "exec" | "compile" => {
            let actual_name = if name == "exec" { "exec" } else { "compile" };
            Some(SinkInfo {
                name: actual_name.to_owned(),
                rule_id: ids::RULE_ID_EXEC.to_owned(),
                vuln_type: VulnType::CodeInjection,
                severity: Severity::Critical,
                dangerous_args: vec![0],
                dangerous_keywords: Vec::new(),
                remediation: format!(
                    "Avoid {actual_name}() with user input. Consider safer alternatives."
                ),
            })
        }
        _ => None,
    }
}

pub(super) fn check_sql_injection_sinks(name: &str) -> Option<SinkInfo> {
    if name.ends_with(".execute") || name.ends_with(".executemany") {
        return Some(SinkInfo {
            name: name.to_owned(),
            rule_id: ids::RULE_ID_SQL_RAW.to_owned(),
            vuln_type: VulnType::SqlInjection,
            severity: Severity::Critical,
            dangerous_args: vec![0],
            dangerous_keywords: Vec::new(),
            remediation: "Use parameterized queries: cursor.execute(sql, (param,))".to_owned(),
        });
    }

    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    if name == "sqlalchemy.text" || name.ends_with(".text") || name.ends_with(".objects.raw") {
        let rule_id = if name.ends_with(".objects.raw")
            || name == "sqlalchemy.text"
            || name.ends_with(".text")
        {
            ids::RULE_ID_SQL_INJECTION.to_owned()
        } else {
            ids::RULE_ID_SQL_RAW.to_owned()
        };
        return Some(SinkInfo {
            name: name.to_owned(),
            rule_id,
            vuln_type: VulnType::SqlInjection,
            severity: Severity::Critical,
            dangerous_args: vec![0],
            dangerous_keywords: Vec::new(),
            remediation: if name.ends_with(".objects.raw") {
                "Use Django ORM methods instead of raw SQL.".to_owned()
            } else {
                "Use bound parameters: text('SELECT * WHERE id=:id').bindparams(id=val)".to_owned()
            },
        });
    }

    if name == "pandas.read_sql"
        || name == "pd.read_sql"
        || name == "Template.substitute"
        || name == "JinjaSql.prepare_query"
    {
        return Some(SinkInfo {
            name: name.to_owned(),
            rule_id: ids::RULE_ID_SQL_RAW.to_owned(),
            vuln_type: VulnType::SqlInjection,
            severity: if name.starts_with("pandas") || name.starts_with("pd") {
                Severity::High
            } else {
                Severity::Critical
            },
            dangerous_args: vec![0],
            dangerous_keywords: Vec::new(),
            remediation: if name.contains("pandas") || name.contains("pd") {
                "Use parameterized queries with pd.read_sql(sql, con, params=[...])".to_owned()
            } else {
                "Avoid building raw SQL strings. Use parameterized queries.".to_owned()
            },
        });
    }

    None
}
