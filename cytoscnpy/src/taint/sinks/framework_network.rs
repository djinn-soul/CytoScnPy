use super::{Severity, SinkInfo, VulnType};
use crate::rules::ids;

pub(super) fn check_network_sinks(name: &str) -> Option<SinkInfo> {
    if name.starts_with("requests.")
        || name.starts_with("httpx.")
        || name == "urllib.request.urlopen"
        || name == "urlopen"
    {
        let dangerous_args = if name.ends_with(".request") {
            vec![1]
        } else {
            vec![0]
        };
        return Some(SinkInfo {
            name: name.to_owned(),
            rule_id: ids::RULE_ID_SSRF.to_owned(),
            vuln_type: VulnType::Ssrf,
            severity: Severity::Critical,
            dangerous_args,
            dangerous_keywords: vec!["url".to_owned(), "uri".to_owned(), "address".to_owned()],
            remediation: "Validate URLs against an allowlist. Block internal/private IP ranges."
                .to_owned(),
        });
    }

    if name == "redirect" || name == "flask.redirect" || name == "django.shortcuts.redirect" {
        return Some(SinkInfo {
            name: name.to_owned(),
            rule_id: ids::RULE_ID_OPEN_REDIRECT.to_owned(),
            vuln_type: VulnType::OpenRedirect,
            severity: Severity::Medium,
            dangerous_args: vec![0],
            dangerous_keywords: Vec::new(),
            remediation: "Validate redirect URLs against an allowlist.".to_owned(),
        });
    }

    None
}

pub(super) fn check_framework_sink_packs(name: &str) -> Option<SinkInfo> {
    if name == "django.http.HttpResponse"
        || name == "HttpResponse"
        || name == "django.http.JsonResponse"
        || name == "JsonResponse"
        || name == "starlette.responses.Response"
        || name == "fastapi.Response"
    {
        return Some(SinkInfo {
            name: name.to_owned(),
            rule_id: ids::RULE_ID_XSS_GENERIC.to_owned(),
            vuln_type: VulnType::Xss,
            severity: Severity::High,
            dangerous_args: vec![0],
            dangerous_keywords: vec!["content".to_owned(), "body".to_owned()],
            remediation: "Escape untrusted content before writing raw HTTP responses.".to_owned(),
        });
    }

    if name == "django.http.HttpResponseRedirect"
        || name == "HttpResponseRedirect"
        || name == "RedirectResponse"
        || name == "starlette.responses.RedirectResponse"
        || name == "fastapi.responses.RedirectResponse"
    {
        return Some(SinkInfo {
            name: name.to_owned(),
            rule_id: ids::RULE_ID_OPEN_REDIRECT.to_owned(),
            vuln_type: VulnType::OpenRedirect,
            severity: Severity::Medium,
            dangerous_args: vec![0],
            dangerous_keywords: vec!["redirect_to".to_owned(), "url".to_owned()],
            remediation: "Validate redirect targets against an allowlist.".to_owned(),
        });
    }

    None
}
