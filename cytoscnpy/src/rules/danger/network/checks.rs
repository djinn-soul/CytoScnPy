use super::super::utils::create_finding;
use super::metadata::{
    META_FTP, META_HTTPS_CONNECTION, META_SSL_UNVERIFIED, META_TELNET, META_URL_OPEN,
    META_WRAP_SOCKET,
};
use crate::rules::{Context, Finding};
use ruff_python_ast as ast;
use ruff_text_size::Ranged;

/// Checks shared network/SSL call patterns and returns a finding when insecure usage is detected.
pub fn check_network_and_ssl(
    name: &str,
    call: &ast::ExprCall,
    context: &Context,
) -> Option<Finding> {
    if name == "httplib.HTTPSConnection"
        || name == "http.client.HTTPSConnection"
        || name == "six.moves.http_client.HTTPSConnection"
    {
        let has_context = call
            .arguments
            .keywords
            .iter()
            .any(|keyword| keyword.arg.as_ref().is_some_and(|arg| arg == "context"));
        if !has_context {
            return Some(create_finding(
                "Use of HTTPSConnection without a context is insecure in some Python versions.",
                META_HTTPS_CONNECTION,
                context,
                call.range().start(),
                "MEDIUM",
            ));
        }
    }
    if name.starts_with("urllib.urlopen")
        || name.starts_with("urllib.request.urlopen")
        || name.starts_with("urllib2.urlopen")
        || name.starts_with("six.moves.urllib.request.urlopen")
        || name.contains("urlretrieve")
        || name.contains("URLopener")
    {
        return Some(create_finding(
            "Audit url open for permitted schemes. Allowing file: or custom schemes is dangerous.",
            META_URL_OPEN,
            context,
            call.range().start(),
            "MEDIUM",
        ));
    }
    if name.starts_with("telnetlib.") {
        return Some(create_finding(
            "Telnet-related functions are being called. Telnet is insecure.",
            META_TELNET,
            context,
            call.range().start(),
            "HIGH",
        ));
    }
    if name.starts_with("ftplib.") {
        return Some(create_finding(
            "FTP-related functions are being called. FTP is insecure.",
            META_FTP,
            context,
            call.range().start(),
            "HIGH",
        ));
    }
    if name == "ssl._create_unverified_context" {
        return Some(create_finding(
            "Use of potentially insecure ssl._create_unverified_context.",
            META_SSL_UNVERIFIED,
            context,
            call.range().start(),
            "MEDIUM",
        ));
    }
    if name == "ssl.wrap_socket" {
        return Some(create_finding("Use of ssl.wrap_socket is deprecated and often insecure. Use ssl.create_default_context().wrap_socket() instead.", META_WRAP_SOCKET, context, call.range().start(), "MEDIUM"));
    }
    None
}
