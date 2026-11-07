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

#[path = "notebook_fix_test/cases_1.rs"]
mod cases_1;
