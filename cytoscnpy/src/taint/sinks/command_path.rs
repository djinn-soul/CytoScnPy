use super::{has_shell_true, Severity, SinkInfo, VulnType};
use crate::rules::ids;
use ruff_python_ast::{self as ast};

pub(super) fn check_command_injection_sinks(name: &str, call: &ast::ExprCall) -> Option<SinkInfo> {
    if name == "os.system"
        || name == "os.popen"
        || (name.starts_with("subprocess.") && has_shell_true(call))
    {
        return Some(SinkInfo {
            name: name.to_owned(),
            rule_id: ids::RULE_ID_SUBPROCESS.to_owned(),
            vuln_type: VulnType::CommandInjection,
            severity: Severity::Critical,
            dangerous_args: vec![0],
            dangerous_keywords: Vec::new(),
            remediation: if name.starts_with("subprocess") {
                "Use shell=False and pass arguments as a list.".to_owned()
            } else {
                "Use subprocess.run() with shell=False.".to_owned()
            },
        });
    }
    None
}

pub(super) fn check_path_traversal_sinks(name: &str) -> Option<SinkInfo> {
    if name == "open" {
        return Some(SinkInfo {
            name: "open".to_owned(),
            rule_id: ids::RULE_ID_PATH_TRAVERSAL.to_owned(),
            vuln_type: VulnType::PathTraversal,
            severity: Severity::High,
            dangerous_args: vec![0],
            dangerous_keywords: Vec::new(),
            remediation: "Validate and sanitize file paths. Use os.path.basename() or pathlib."
                .to_owned(),
        });
    }

    if name.starts_with("shutil.") || name.starts_with("os.path.") {
        return Some(SinkInfo {
            name: name.to_owned(),
            rule_id: "CSP-D501".to_owned(),
            vuln_type: VulnType::PathTraversal,
            severity: Severity::High,
            dangerous_args: vec![],
            dangerous_keywords: vec!["path".to_owned(), "src".to_owned(), "dst".to_owned()],
            remediation: "Validate file paths before file operations.".to_owned(),
        });
    }

    let is_pathlib = name == "pathlib.Path"
        || name == "pathlib.PurePath"
        || name == "pathlib.PosixPath"
        || name == "pathlib.WindowsPath"
        || name == "Path"
        || name == "PurePath"
        || name == "PosixPath"
        || name == "WindowsPath"
        || name == "zipfile.Path";

    if is_pathlib {
        return Some(SinkInfo {
            name: name.to_owned(),
            rule_id: "CSP-D501".to_owned(),
            vuln_type: VulnType::PathTraversal,
            severity: Severity::High,
            dangerous_args: vec![0],
            dangerous_keywords: vec![
                "path".to_owned(),
                "at".to_owned(),
                "file".to_owned(),
                "filename".to_owned(),
                "filepath".to_owned(),
            ],
            remediation: "Validate and sanitize file paths. Use os.path.basename() or pathlib."
                .to_owned(),
        });
    }

    None
}
