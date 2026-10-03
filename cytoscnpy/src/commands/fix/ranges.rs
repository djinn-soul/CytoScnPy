use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::Stmt;
use ruff_text_size::Ranged;

pub(super) struct MethodEdit {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) class_would_be_empty: bool,
}

pub(super) fn find_def_range(
    body: &[Stmt],
    name: &str,
    def_type: &str,
    target_start_byte: Option<usize>,
) -> Option<(usize, usize)> {
    if def_type == "method" {
        return find_method_edit(body, name, target_start_byte).map(|edit| (edit.start, edit.end));
    }
    let mut finder = RangeFinder {
        name,
        def_type,
        target: target_start_byte,
        range: None,
    };
    finder.visit_body(body);
    finder.range
}

struct RangeFinder<'a> {
    name: &'a str,
    def_type: &'a str,
    target: Option<usize>,
    range: Option<(usize, usize)>,
}

impl<'a> Visitor<'a> for RangeFinder<'_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if self.range.is_some() {
            return;
        }
        let candidate = match stmt {
            Stmt::FunctionDef(node)
                if self.def_type == "function" && node.name.as_str() == self.name =>
            {
                Some((node.range(), node.name.range(), &node.decorator_list))
            }
            Stmt::ClassDef(node) if self.def_type == "class" && node.name.as_str() == self.name => {
                Some((node.range(), node.name.range(), &node.decorator_list))
            }
            _ => None,
        };
        if let Some((range, name, decorators)) = candidate {
            let start = decorators
                .iter()
                .map(|d| d.range().start().to_usize())
                .min()
                .unwrap_or(range.start().to_usize())
                .min(range.start().to_usize());
            if self.target.map_or(true, |target| {
                target == start
                    || target == range.start().to_usize()
                    || target == name.start().to_usize()
            }) {
                self.range = Some((start, range.end().to_usize()));
                return;
            }
        }
        // Preserve the legacy whole-statement import lookup for internal callers.
        let import_match = match stmt {
            Stmt::Import(node) if self.def_type == "import" => node
                .names
                .iter()
                .any(|alias| alias.asname.as_ref().unwrap_or(&alias.name).as_str() == self.name),
            Stmt::ImportFrom(node) if self.def_type == "import" && node.names.len() == 1 => {
                node.names[0]
                    .asname
                    .as_ref()
                    .unwrap_or(&node.names[0].name)
                    .as_str()
                    == self.name
            }
            _ => false,
        };
        if import_match {
            self.range = Some((
                stmt.range().start().to_usize(),
                stmt.range().end().to_usize(),
            ));
        } else {
            visitor::walk_stmt(self, stmt);
        }
    }
}

pub(super) fn find_method_edit(
    body: &[Stmt],
    name: &str,
    target_start_byte: Option<usize>,
) -> Option<MethodEdit> {
    let mut finder = MethodFinder {
        name,
        target: target_start_byte,
        edit: None,
    };
    finder.visit_body(body);
    finder.edit
}

struct MethodFinder<'a> {
    name: &'a str,
    target: Option<usize>,
    edit: Option<MethodEdit>,
}

impl<'a> Visitor<'a> for MethodFinder<'_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if self.edit.is_some() {
            return;
        }
        if let Stmt::ClassDef(class) = stmt {
            for class_stmt in &class.body {
                if let Stmt::FunctionDef(func) = class_stmt {
                    let start = func
                        .decorator_list
                        .iter()
                        .map(|d| d.range().start().to_usize())
                        .min()
                        .unwrap_or(func.range().start().to_usize())
                        .min(func.range().start().to_usize());
                    if func.name.as_str() == self.name
                        && self.target.map_or(true, |target| {
                            target == start
                                || target == func.range().start().to_usize()
                                || target == func.name.range().start().to_usize()
                        })
                    {
                        self.edit = Some(MethodEdit {
                            start,
                            end: func.range().end().to_usize(),
                            class_would_be_empty: class.body.len() == 1,
                        });
                        return;
                    }
                }
            }
        }
        visitor::walk_stmt(self, stmt);
    }
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
