//! Immutable source and AST snapshots shared by a single analysis invocation.
use ruff_python_ast::ModModule;
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub(crate) struct ParsedSource {
    pub content: String,
    pub module: ModModule,
}

pub(crate) type SourceCache = HashMap<PathBuf, Result<ParsedSource, String>>;

pub(crate) fn load_source<'a>(
    path: &Path,
    sources: Option<&'a SourceCache>,
) -> Result<Cow<'a, ParsedSource>, String> {
    if let Some(sources) = sources {
        return sources
            .get(path)
            .ok_or_else(|| format!("{}: missing source inventory entry", path.display()))?
            .as_ref()
            .map(Cow::Borrowed)
            .map_err(Clone::clone);
    }
    let content = std::fs::read_to_string(path)
        .map_err(|error| format!("{}: read error: {error}", path.display()))?;
    let module = ruff_python_parser::parse_module(&content)
        .map_err(|error| format!("{}: Python parse error: {error}", path.display()))?
        .into_syntax();
    Ok(Cow::Owned(ParsedSource { content, module }))
}

pub(crate) fn is_minified(path: &Path, sources: Option<&SourceCache>) -> bool {
    if let Some(sources) = sources {
        let content = sources
            .get(path)
            .and_then(|source| source.as_ref().ok())
            .map(|source| source.content.as_str());
        crate::utils::is_likely_minified_path(path)
            || content.is_some_and(crate::utils::is_likely_minified_content)
    } else {
        crate::utils::is_likely_minified(path, None)
    }
}

#[cfg(test)]
#[path = "sources_tests.rs"]
mod tests;
