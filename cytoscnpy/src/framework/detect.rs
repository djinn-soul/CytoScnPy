use super::visitor::FrameworkAwareVisitor;

/// Detects framework usage for a given definition.
#[must_use]
pub fn detect_framework_usage(
    line: usize,
    simple_name: &str,
    def_type: &str,
    visitor: Option<&FrameworkAwareVisitor>,
) -> Option<u8> {
    let visitor = visitor?;
    if def_type != "function" && def_type != "method" {
        return None;
    }
    if !visitor.is_framework_file {
        return None;
    }
    if simple_name.starts_with('_') && !simple_name.starts_with("__") {
        return None;
    }
    if visitor.framework_decorated_lines.contains(&line) {
        return Some(100);
    }
    None
}
