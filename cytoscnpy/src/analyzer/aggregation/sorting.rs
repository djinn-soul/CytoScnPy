use crate::analyzer::types::ParseError;
use crate::rules::secrets::SecretFinding;
use crate::rules::Finding;
use crate::taint::types::TaintFinding;
use crate::visitor::Definition;

pub(super) fn sort_definitions(definitions: &mut [Definition]) {
    definitions.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.line.cmp(&b.line))
            .then(a.col.cmp(&b.col))
    });
}

pub(super) fn sort_findings(findings: &mut [Finding]) {
    findings.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.line.cmp(&b.line))
            .then(a.col.cmp(&b.col))
    });
}

pub(super) fn sort_secrets(findings: &mut [SecretFinding]) {
    findings.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));
}

pub(super) fn sort_parse_errors(errors: &mut [ParseError]) {
    errors.sort_by(|a, b| a.file.cmp(&b.file).then(a.error.cmp(&b.error)));
}

pub(super) fn sort_taint_findings(findings: &mut [TaintFinding]) {
    findings.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.sink_line.cmp(&b.sink_line))
            .then(a.sink_col.cmp(&b.sink_col))
    });
}
