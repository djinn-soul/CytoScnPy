//! `file:` URIs to filesystem paths and back.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The local path a `file:` URI names, or `None` for other schemes
/// (`untitled:` buffers, remote files) and remote hosts.
pub(crate) fn to_path(uri: &str) -> Option<PathBuf> {
    to_path_for(uri, cfg!(windows))
}

fn to_path_for(uri: &str, windows: bool) -> Option<PathBuf> {
    let rest = uri
        .get(..7)
        .filter(|scheme| scheme.eq_ignore_ascii_case("file://"))
        .map(|_| &uri[7..])?;
    let path_start = rest.find('/')?;
    let host = &rest[..path_start];
    if !(host.is_empty() || host.eq_ignore_ascii_case("localhost")) {
        return None;
    }
    let raw = &rest[path_start..];
    let raw = raw.find(['?', '#']).map_or(raw, |end| &raw[..end]);
    let decoded = percent_decode(raw)?;
    if windows {
        // `/C:/x` names `C:/x`; `\` separators are fine on Windows.
        let bytes = decoded.as_bytes();
        let has_drive = bytes.len() >= 3 && bytes[1].is_ascii_alphabetic() && bytes[2] == b':';
        if has_drive {
            return Some(PathBuf::from(&decoded[1..]));
        }
    }
    Some(PathBuf::from(decoded))
}

/// The `file:` URI of an absolute local path.
pub(crate) fn from_path(path: &Path) -> Option<lsp_types::Uri> {
    from_path_for(&path.to_string_lossy(), cfg!(windows))?
        .parse()
        .ok()
}

fn from_path_for(path: &str, windows: bool) -> Option<String> {
    let path = if windows {
        // `canonicalize` names `C:\x` as `\\?\C:\x`.
        let path = path.strip_prefix(r"\\?\").unwrap_or(path);
        format!("/{}", path.replace('\\', "/"))
    } else {
        path.to_owned()
    };
    if !path.starts_with('/') {
        return None;
    }
    let mut uri = String::from("file://");
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~:".contains(&byte) {
            uri.push(char::from(byte));
        } else {
            write!(uri, "%{byte:02X}").ok()?;
        }
    }
    Some(uri)
}

fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = text.get(i + 1..i + 3)?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

#[cfg(test)]
mod tests {
    use super::{from_path_for, to_path_for};
    use std::path::PathBuf;

    #[test]
    fn decodes_unix_file_uris() {
        assert_eq!(
            to_path_for("file:///home/me/my%20project/caf%C3%A9.py", false),
            Some(PathBuf::from("/home/me/my project/café.py"))
        );
        assert_eq!(
            to_path_for("FILE://localhost/tmp/a.py", false),
            Some(PathBuf::from("/tmp/a.py"))
        );
    }

    #[test]
    fn strips_the_slash_before_a_windows_drive() {
        assert_eq!(
            to_path_for("file:///c%3A/Users/me/a.py", true),
            Some(PathBuf::from("c:/Users/me/a.py"))
        );
        assert_eq!(
            to_path_for("file:///C:/a.py", false),
            Some(PathBuf::from("/C:/a.py"))
        );
    }

    #[test]
    fn rejects_other_schemes_hosts_and_bad_escapes() {
        assert_eq!(to_path_for("untitled:Untitled-1", false), None);
        assert_eq!(to_path_for("file://server/share/a.py", false), None);
        assert_eq!(to_path_for("file:///a%2.py", false), None);
        assert_eq!(to_path_for("file:///a%ff.py", false), None);
    }

    #[test]
    fn encodes_paths_as_file_uris_that_decode_back() {
        let uri = from_path_for("/home/me/my project/café.py", false);
        assert_eq!(
            uri.as_deref(),
            Some("file:///home/me/my%20project/caf%C3%A9.py")
        );
        assert_eq!(
            uri.and_then(|uri| to_path_for(&uri, false)),
            Some(PathBuf::from("/home/me/my project/café.py"))
        );
        assert_eq!(
            from_path_for(r"\\?\C:\Users\me\a.py", true).as_deref(),
            Some("file:///C:/Users/me/a.py")
        );
        assert_eq!(from_path_for("relative/a.py", false), None);
    }
}
