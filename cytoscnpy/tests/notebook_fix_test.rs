//! Dead-code fixes in notebooks are planned per cell and written back to the notebook.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use cytoscnpy::analyzer::{AnalysisResult, CytoScnPy};
use cytoscnpy::commands::{run_fix_deadcode, verify_fix_suggestions, DeadCodeFixOptions};
use std::path::Path;

const NOTEBOOK: &str = r##"{
 "cells": [
  {
   "cell_type": "markdown",
   "id": "md",
   "metadata": {},
   "source": ["# Title"]
  },
  {
   "cell_type": "code",
   "execution_count": null,
   "id": "c1",
   "metadata": {},
   "outputs": [],
   "source": ["import os, sys\n", "print(sys)"]
  },
  {
   "cell_type": "code",
   "execution_count": null,
   "id": "c2",
   "metadata": {},
   "outputs": [],
   "source": "class Box:\n    def m1(self):\n        pass\n    def m2(self):\n        pass\n\nBox()"
  }
 ],
 "metadata": {},
 "nbformat": 4,
 "nbformat_minor": 5
}
"##;

fn analyze(dir: &Path, cell_level: bool) -> AnalysisResult {
    let mut analyzer = CytoScnPy::default()
        .with_confidence(80)
        .with_tests(false)
        .with_ipynb(true);
    let mut result = analyzer.analyze(dir);
    if cell_level {
        cytoscnpy::ipynb::assign_cells(&mut result, Some(dir));
    }
    result
}

fn cell_source(notebook: &serde_json::Value, index: usize) -> String {
    match &notebook["cells"][index]["source"] {
        serde_json::Value::String(source) => source.clone(),
        lines => lines
            .as_array()
            .unwrap()
            .iter()
            .map(|line| line.as_str().unwrap())
            .collect(),
    }
}

fn apply(source: &str, start: usize, end: usize, replacement: &str) -> String {
    format!("{}{replacement}{}", &source[..start], &source[end..])
}

#[test]
fn findings_carry_their_cell_and_cell_relative_lines() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("n.ipynb"), NOTEBOOK).unwrap();

    let result = analyze(dir.path(), true);
    let os = result
        .unused_imports
        .iter()
        .find(|def| def.simple_name == "os")
        .unwrap();
    assert_eq!((os.cell_number, os.line), (Some(1), 1));
    let m2 = result
        .unused_methods
        .iter()
        .find(|def| def.simple_name == "m2")
        .unwrap();
    assert_eq!((m2.cell_number, m2.line), (Some(2), 4));

    // Without --ipynb-cells, lines keep counting through the joined cells,
    // and no finding claims a cell its line is not relative to.
    let joined = analyze(dir.path(), false);
    let m2 = joined
        .unused_methods
        .iter()
        .find(|def| def.simple_name == "m2")
        .unwrap();
    assert_eq!((m2.cell_number, m2.line), (None, 7));
}

#[test]
fn suggested_fixes_are_relative_to_their_cell_and_keep_it_valid() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("n.ipynb"), NOTEBOOK).unwrap();
    let notebook: serde_json::Value = serde_json::from_str(NOTEBOOK).unwrap();

    let mut result = analyze(dir.path(), true);
    verify_fix_suggestions(&mut result, Some(dir.path()));

    let os = result
        .unused_imports
        .iter()
        .find(|def| def.simple_name == "os")
        .unwrap();
    let fix = os.fix.as_deref().expect("os should have a fix");
    assert_eq!(fix.cell_number, Some(1), "offsets are relative to cell 1");
    let fixed = apply(
        &cell_source(&notebook, 1),
        fix.start_byte,
        fix.end_byte,
        &fix.replacement,
    );
    assert_eq!(fixed, "import sys\nprint(sys)");

    // Removing both methods together keeps the class body.
    let plan = result
        .unused_methods
        .iter()
        .find_map(|def| {
            def.fix
                .as_deref()
                .filter(|fix| !fix.rule_fix_all.is_empty())
                .map(|fix| fix.rule_fix_all.clone())
        })
        .expect("two unused methods in one cell should carry a combined plan");
    let mut fixed = cell_source(&notebook, 2);
    let mut edits = plan;
    edits.sort_by_key(|edit| std::cmp::Reverse(edit.start_byte));
    for edit in edits {
        fixed = apply(&fixed, edit.start_byte, edit.end_byte, &edit.replacement);
    }
    ruff_python_parser::parse_module(&fixed).expect("cell should stay valid");
    assert!(fixed.starts_with("class Box:\n    pass"), "{fixed}");
}

#[test]
fn fix_command_rewrites_only_the_changed_cells() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("n.ipynb");
    std::fs::write(&path, NOTEBOOK).unwrap();

    let result = analyze(dir.path(), false);
    let options = DeadCodeFixOptions {
        min_confidence: 80,
        fix_functions: true,
        fix_methods: true,
        fix_classes: true,
        fix_imports: true,
        fix_variables: true,
        with_cst: true,
        analysis_root: dir.path().to_path_buf(),
        ..DeadCodeFixOptions::default()
    };
    run_fix_deadcode(&result, &options, std::io::sink()).unwrap();

    let written = std::fs::read_to_string(&path).unwrap();
    let notebook: serde_json::Value = serde_json::from_str(&written).unwrap();
    assert_eq!(
        notebook["cells"][1]["source"],
        serde_json::json!(["import sys\n", "print(sys)"]),
        "list sources stay lists of lines"
    );
    let class_cell = cell_source(&notebook, 2);
    assert!(
        notebook["cells"][2]["source"].is_string(),
        "string sources stay strings"
    );
    ruff_python_parser::parse_module(&class_cell).expect("cell should stay valid");
    assert!(
        class_cell.starts_with("class Box:\n    pass"),
        "{class_cell}"
    );
    // Everything but the sources is untouched, layout included.
    let restore = |text: &str| {
        let mut value: serde_json::Value = serde_json::from_str(text).unwrap();
        for cell in value["cells"].as_array_mut().unwrap() {
            cell["source"] = serde_json::Value::Null;
        }
        value
    };
    assert_eq!(restore(&written), restore(NOTEBOOK));
    assert!(written.starts_with("{\n \"cells\": [\n  {\n   \"cell_type\""));
}

#[test]
fn security_quality_and_taint_findings_map_to_their_cells() {
    let cells = [
        "import subprocess",
        "AWS_SECRET_ACCESS_KEY = 'wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY'\nprint(1)",
        "def run(cmd):\n    subprocess.call(cmd, shell=True)\n\nrun('ls')",
        "user = input()",
        "x = 1\neval(user)",
        "def many(a, b, c, d, e, f, g, h, i):\n    return a\nmany(*range(9))",
    ];
    let notebook = serde_json::json!({
        "cells": cells.iter().enumerate().map(|(index, source)| serde_json::json!({
            "cell_type": "code",
            "execution_count": null,
            "id": format!("c{index}"),
            "metadata": {},
            "outputs": [],
            "source": source,
        })).collect::<Vec<_>>(),
        "metadata": {},
        "nbformat": 4,
        "nbformat_minor": 5,
    });
    let dir = tempfile::tempdir().unwrap();
    // Compact single-line JSON, as some tools write it: still a notebook,
    // not a minified file.
    std::fs::write(dir.path().join("n.ipynb"), notebook.to_string()).unwrap();

    let mut analyzer = CytoScnPy::default()
        .with_confidence(80)
        .with_tests(false)
        .with_ipynb(true)
        .with_secrets(true)
        .with_danger(true)
        .with_quality(true);
    let mut result = analyzer.analyze(dir.path());
    cytoscnpy::ipynb::assign_cells(&mut result, Some(dir.path()));

    let line_of = |cell: Option<usize>, line: usize| {
        cells[cell.expect("finding should have a cell")]
            .lines()
            .nth(line - 1)
            .unwrap()
            .trim()
    };
    let shell = result
        .danger
        .iter()
        .find(|finding| finding.rule_id == "CSP-D003")
        .expect("shell=True should be reported");
    assert_eq!(
        line_of(shell.cell_number, shell.line),
        "subprocess.call(cmd, shell=True)"
    );
    let secret = result
        .secrets
        .first()
        .expect("the AWS key should be reported");
    assert_eq!(secret.cell_number, Some(1));
    assert_eq!(secret.line, 1);
    let quality = result
        .quality
        .first()
        .expect("too many arguments should be reported");
    assert_eq!(quality.cell_number, Some(5));
    assert!(line_of(quality.cell_number, quality.line).starts_with("def many("));

    // The tainted value flows from one cell into a sink in the next.
    let eval = result
        .taint_findings
        .iter()
        .find(|finding| finding.sink.contains("eval"))
        .expect("input() reaching eval should be reported");
    assert_eq!(line_of(eval.cell_number, eval.sink_line), "eval(user)");
    assert_eq!(
        line_of(eval.source_cell_number, eval.source_line),
        "user = input()"
    );
}

#[test]
fn writing_cells_leaves_the_rest_of_the_notebook_byte_identical() {
    let original = r#"{
 "cells": [
  {
   "cell_type": "code",
   "execution_count": 1,
   "id": "c0",
   "metadata": {"scrolled": 1e-05},
   "outputs": [{"data": {"text/plain": ["0.1"]}, "metadata": {"w": 1.0E+3}, "output_type": "execute_result"}],
   "source": [
    "import os\n",
    "x = 1"
   ]
  },
  {
   "cell_type": "code",
   "execution_count": null,
   "id": "c1",
   "metadata": {},
   "outputs": [],
   "source": "import sys\ny = 2"
  }
 ],
 "metadata": {"version": 3.10},
 "nbformat": 4,
 "nbformat_minor": 5
}
"#;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nb.ipynb");
    std::fs::write(&path, original).unwrap();

    cytoscnpy::ipynb::write_cell_sources(
        &path,
        Some(dir.path()),
        &[(0, "x = 1".to_owned()), (1, "y = 2".to_owned())],
    )
    .unwrap();

    let expected = original
        .replace(
            "   \"source\": [\n    \"import os\\n\",\n    \"x = 1\"\n   ]",
            "   \"source\": [\n    \"x = 1\"\n   ]",
        )
        .replace("\"import sys\\ny = 2\"", "\"y = 2\"");
    assert_ne!(expected, original);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), expected);
}
