//! Python-specific line heuristics shared by the TODO scanner.

/// Returns `true` when the source is a command-line script whose prints are output.
pub(super) fn is_script_source(content: &str) -> bool {
    content.lines().any(|line| {
        let t = line.trim();
        is_main_guard(t)
            || ["argparse", "click", "typer", "fire"].iter().any(|module| {
                t.strip_prefix("import ")
                    .or_else(|| t.strip_prefix("from "))
                    .is_some_and(|rest| first_module(rest) == *module)
            })
    })
}

fn is_main_guard(line: &str) -> bool {
    let compact: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    compact.starts_with("if__name__==\"__main__\"") || compact.starts_with("if__name__=='__main__'")
}

fn first_module(rest: &str) -> &str {
    rest.split(|c: char| c == '.' || c == ',' || c.is_whitespace())
        .next()
        .unwrap_or("")
}

fn is_dotted_name(text: &str) -> bool {
    !text.is_empty()
        && text.split('.').all(|part| {
            let mut chars = part.chars();
            chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
                && chars.all(|c| c.is_alphanumeric() || c == '_')
        })
}

/// Checks `name` or `name as alias` entries separated by commas.
fn is_import_list(text: &str, allow_dotted: bool) -> bool {
    let text = text.trim();
    let text = text
        .strip_prefix('(')
        .map_or(text, |inner| inner.trim_end_matches(')'));
    if text.trim() == "*" {
        return !allow_dotted;
    }
    text.split(',').map(str::trim).all(|entry| {
        let mut words = entry.split_whitespace();
        let valid_name = |w: Option<&str>| {
            w.is_some_and(|w| is_dotted_name(w) && (allow_dotted || !w.contains('.')))
        };
        match (words.next(), words.next(), words.next(), words.next()) {
            (name, None, None, None) => valid_name(name),
            (name, Some("as"), Some(alias), None) => valid_name(name) && is_dotted_name(alias),
            _ => false,
        }
    })
}

/// Returns `true` when `body` is a complete Python import statement.
///
/// Prose such as `import X pattern` fails because entries must be names.
pub(super) fn is_import_statement(body: &str) -> bool {
    if let Some(rest) = body.strip_prefix("import ") {
        return is_import_list(rest, true);
    }
    let Some(rest) = body.strip_prefix("from ") else {
        return false;
    };
    let Some((module, names)) = rest.split_once(" import ") else {
        return false;
    };
    let module = module.trim().trim_start_matches('.');
    (module.is_empty() || is_dotted_name(module)) && is_import_list(names, false)
}
