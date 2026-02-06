//! Tests for LCOM4 exemptions on framework/model-like classes.
#![allow(clippy::expect_used)]

use cytoscnpy::config::Config;
use cytoscnpy::linter::LinterVisitor;
use cytoscnpy::rules::ids::RULE_ID_COHESION;
use cytoscnpy::rules::quality::get_quality_rules;
use cytoscnpy::utils::LineIndex;
use ruff_python_parser::{parse, Mode};
use std::path::PathBuf;

fn run_quality(source: &str) -> Vec<cytoscnpy::rules::Finding> {
    let tree = parse(source, Mode::Module.into()).expect("Failed to parse");
    let line_index = LineIndex::new(source);
    let rules = get_quality_rules(&Config::default());
    let mut linter = LinterVisitor::new(
        rules,
        PathBuf::from("test.py"),
        line_index,
        Config::default(),
    );

    if let ruff_python_ast::Mod::Module(module) = tree.into_syntax() {
        for stmt in &module.body {
            linter.visit_stmt(stmt);
        }
    }
    linter.findings
}

fn assert_no_cohesion_findings(source: &str) {
    let findings = run_quality(source);
    assert!(
        !findings.iter().any(|f| f.rule_id == RULE_ID_COHESION),
        "Expected no LCOM4 findings, got: {findings:?}"
    );
}

#[test]
fn test_lcom4_skips_protocol_class() {
    let source = r"
from typing import Protocol

class Sample(Protocol):
    def a(self):
        pass

    def b(self):
        pass
";
    assert_no_cohesion_findings(source);
}

#[test]
fn test_lcom4_skips_pydantic_model() {
    let source = r"
from pydantic import BaseModel

class User(BaseModel):
    def get_name(self):
        return self.name

    def get_age(self):
        return self.age
";
    assert_no_cohesion_findings(source);
}

#[test]
fn test_lcom4_skips_dataclass() {
    let source = r"
from dataclasses import dataclass

@dataclass
class Person:
    name: str
    age: int

    def get_name(self):
        return self.name

    def get_age(self):
        return self.age
";
    assert_no_cohesion_findings(source);
}

#[test]
fn test_lcom4_skips_attrs_define() {
    let source = r"
import attrs

@attrs.define
class Item:
    price: int
    tax: int

    def price_value(self):
        return self.price

    def tax_value(self):
        return self.tax
";
    assert_no_cohesion_findings(source);
}

#[test]
fn test_lcom4_skips_attrs_mutable() {
    let source = r"
import attrs

@attrs.mutable
class Mutable:
    x: int
    y: int

    def get_x(self):
        return self.x

    def get_y(self):
        return self.y
";
    assert_no_cohesion_findings(source);
}

#[test]
fn test_lcom4_skips_pydantic_dataclass() {
    let source = r"
import pydantic

@pydantic.dataclasses.dataclass
class Item:
    name: str
    price: int

    def get_name(self):
        return self.name

    def get_price(self):
        return self.price
";
    assert_no_cohesion_findings(source);
}
