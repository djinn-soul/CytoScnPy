use ruff_python_ast::Stmt;
use ruff_text_size::Ranged;

#[derive(Debug, Clone, Copy)]
pub(super) enum ImportEdit {
    DeleteStmt(usize, usize),
    DeleteAlias(usize, usize),
}

pub(super) fn find_def_range(body: &[Stmt], name: &str, def_type: &str) -> Option<(usize, usize)> {
    for stmt in body {
        match stmt {
            Stmt::FunctionDef(f) if def_type == "function" => {
                if f.name.as_str() == name {
                    let start = f.range().start().to_usize();
                    let start = f
                        .decorator_list
                        .iter()
                        .map(|d| d.range().start().to_usize())
                        .min()
                        .unwrap_or(start)
                        .min(start);
                    return Some((start, f.range().end().to_usize()));
                }
            }
            Stmt::ClassDef(c) if def_type == "class" => {
                if c.name.as_str() == name {
                    let start = c.range().start().to_usize();
                    let start = c
                        .decorator_list
                        .iter()
                        .map(|d| d.range().start().to_usize())
                        .min()
                        .unwrap_or(start)
                        .min(start);
                    return Some((start, c.range().end().to_usize()));
                }
            }
            Stmt::Import(i) if def_type == "import" => {
                for alias in &i.names {
                    let import_name = alias.asname.as_ref().unwrap_or(&alias.name);
                    if import_name.as_str() == name {
                        return Some((i.range().start().to_usize(), i.range().end().to_usize()));
                    }
                }
            }
            Stmt::ImportFrom(i) if def_type == "import" => {
                for alias in &i.names {
                    let import_name = alias.asname.as_ref().unwrap_or(&alias.name);
                    if import_name.as_str() == name && i.names.len() == 1 {
                        return Some((i.range().start().to_usize(), i.range().end().to_usize()));
                    }
                }
            }
            _ => {}
        }
    }
    None
}

pub(super) fn find_import_edit(body: &[Stmt], name: &str, source: &str) -> Option<ImportEdit> {
    for stmt in body {
        match stmt {
            Stmt::Import(i) => {
                let names = &i.names;
                for alias in names {
                    let import_name = alias.asname.as_ref().unwrap_or(&alias.name);
                    if import_name.as_str() == name {
                        if names.len() == 1 {
                            let range = i.range();
                            return Some(ImportEdit::DeleteStmt(
                                range.start().to_usize(),
                                range.end().to_usize(),
                            ));
                        }
                        let range = alias.range();
                        let (start, end, has_comma) = trim_comma_range(
                            source,
                            range.start().to_usize(),
                            range.end().to_usize(),
                        );
                        let (start, end) = if has_comma {
                            (start, end)
                        } else {
                            (range.start().to_usize(), range.end().to_usize())
                        };
                        return Some(ImportEdit::DeleteAlias(start, end));
                    }
                }
            }
            Stmt::ImportFrom(i) => {
                let names = &i.names;
                for alias in names {
                    let import_name = alias.asname.as_ref().unwrap_or(&alias.name);
                    if import_name.as_str() == name {
                        if names.len() == 1 {
                            let range = i.range();
                            return Some(ImportEdit::DeleteStmt(
                                range.start().to_usize(),
                                range.end().to_usize(),
                            ));
                        }
                        let range = alias.range();
                        let (start, end, has_comma) = trim_comma_range(
                            source,
                            range.start().to_usize(),
                            range.end().to_usize(),
                        );
                        let (start, end) = if has_comma {
                            (start, end)
                        } else {
                            (range.start().to_usize(), range.end().to_usize())
                        };
                        return Some(ImportEdit::DeleteAlias(start, end));
                    }
                }
            }
            _ => {}
        }
    }
    None
}

pub(super) fn trim_comma_range(source: &str, start: usize, end: usize) -> (usize, usize, bool) {
    let bytes = source.as_bytes();
    let len = bytes.len();

    let mut after = end;
    while after < len && bytes[after].is_ascii_whitespace() {
        after += 1;
    }
    if after < len && bytes[after] == b',' {
        after += 1;
        while after < len && bytes[after].is_ascii_whitespace() {
            after += 1;
        }
        return (start, after, true);
    }

    let mut before = start;
    while before > 0 && bytes[before - 1].is_ascii_whitespace() {
        before -= 1;
    }
    if before > 0 && bytes[before - 1] == b',' {
        before -= 1;
        while before > 0 && bytes[before - 1].is_ascii_whitespace() {
            before -= 1;
        }
        return (before, end, true);
    }

    (start, end, false)
}
