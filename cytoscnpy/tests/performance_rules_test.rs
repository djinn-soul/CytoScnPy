//! Unit tests for performance rules
#![allow(clippy::expect_used)]

use cytoscnpy::config::Config;
use cytoscnpy::linter::LinterVisitor;
use cytoscnpy::rules::quality::get_quality_rules;
use cytoscnpy::utils::LineIndex;
use ruff_python_parser::{parse, Mode};
use std::path::PathBuf;

fn run_linter(source: &str, config: Config) -> LinterVisitor {
    let tree = parse(source, Mode::Module.into()).expect("Failed to parse");
    let line_index = LineIndex::new(source);
    let rules = get_quality_rules(&config);
    let mut linter = LinterVisitor::new(
        rules,
        PathBuf::from("performance_test.py"),
        line_index,
        config,
    );

    if let ruff_python_ast::Mod::Module(module) = tree.into_syntax() {
        for stmt in &module.body {
            linter.visit_stmt(stmt);
        }
    }
    linter
}

#[test]
fn test_membership_list() {
    let source = r"
if x in [1, 2, 3]: pass  # Good (small literal)
if z in [1, 2, 3, 4]: pass  # Bad (larger literal)
for i in items:
    if y in [1, 2, 3]: pass  # Bad (loop)
    if t in [a for a in range(10)]: pass  # Bad (loop + list comp)
";
    let linter = run_linter(source, Config::default());
    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.rule_id == "CSP-P001")
        .collect();
    assert_eq!(
        findings.len(),
        3,
        "Should detect list membership when looped or larger"
    );
}

#[test]
fn test_file_read_memory_risk() {
    let source = r"
with open('large.txt') as f:
    data = f.read()       # Bad (CSP-P002)
    lines = f.readlines() # Bad (CSP-P002)
    chunk = f.read(1024)  # Good
    lines2 = f.readlines(10) # Good

import io
buf = io.StringIO('data')
buf.read() # Good (in-memory)
";
    let linter = run_linter(source, Config::default());
    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.rule_id == "CSP-P002")
        .collect();
    assert_eq!(
        findings.len(),
        2,
        "Should detect potential memory risks with .read() and .readlines()"
    );
}

#[test]
fn test_string_concat_in_loop() {
    let source = r#"
s = ''
for x in items:
    s += x # Bad
    s = s + "!" # Bad

i = 0
while i < 10:
    s += str(i) # Bad
    i += 1

x = 1
for y in items:
    x += y # Good (int)
"#;
    let linter = run_linter(source, Config::default());
    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.rule_id == "CSP-P003")
        .collect();
    assert_eq!(
        findings.len(),
        3,
        "Should detect accumulated + in loops for strings"
    );
}

#[test]
fn test_useless_cast() {
    let source = r"
list(range(10)) # Bad
tuple(map(str, items)) # Bad
list(filter(None, items)) # Bad
list([1, 2, 3]) # Good (casting list to list is useless but rule targets iterators)
                # Rule only checks range/map/filter.
";
    let linter = run_linter(source, Config::default());
    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.rule_id == "CSP-P004")
        .collect();
    assert_eq!(
        findings.len(),
        3,
        "Should detect useless casts on iterators"
    );
}

#[test]
fn test_regex_loop() {
    let source = r"
import re
import ast

for x in items:
    re.compile('foo') # Bad
    re.search('bar', x) # Good (no compile warning)
    ast.parse('x=1') # Bad
    
    re.search(pattern, x) # Good (variable pattern)
    re.compile('foo', flags=re.I) # Bad
";
    let linter = run_linter(source, Config::default());
    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.rule_id == "CSP-P005")
        .collect();
    assert_eq!(
        findings.len(),
        3,
        "Should detect regex compilation/ast in loops"
    );
}

#[test]
fn test_attribute_hoisting() {
    let source = r"
for x in items:
    val = self.config.user.name  # Bad (depth 3)
    val = obj.attr.subattr     # Good (depth 2)
    val = obj.attr             # Good (depth 1)
";
    let linter = run_linter(source, Config::default());
    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.rule_id == "CSP-P006")
        .collect();
    assert_eq!(
        findings.len(),
        1,
        "Should detect deeper attribute chains in loop"
    );
}

#[test]
fn test_pure_call_hoisting() {
    let source = r"
for x in items:
    n = len(data)  # Bad
    y = abs(10)    # Bad
    z = min(1, 2)  # Bad
    a = len(x)     # Good (loop target)
    tmp = [x]
    m = len(tmp)   # Good (assigned in loop)
";
    let linter = run_linter(source, Config::default());
    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.rule_id == "CSP-P007")
        .collect();
    assert_eq!(
        findings.len(),
        3,
        "Should detect pure calls with invariant arguments in loop"
    );
}

#[test]
fn test_exception_flow_loop() {
    let source = r"
for key in keys:
    try:
        val = data[key]
    except KeyError:  # Bad - use .get()
        val = None
        
    try:
        do_work()
    except ValueError: # Good (not a target exception)
        pass
        
    try:
        getattr(obj, 'x')
    except AttributeError: # Bad - use getattr default or hasattr
        pass
";
    let linter = run_linter(source, Config::default());
    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.rule_id == "CSP-P008")
        .collect();
    assert_eq!(
        findings.len(),
        2,
        "Should detect try-except control flow in loop"
    );
}

#[test]
fn test_incorrect_dict_iterator() {
    let source = r"
d = {'a': 1, 'b': 2}
for _, v in d.items(): pass # Bad
for k, _ in d.items(): pass # Bad
for k, v in d.items(): pass # Good
for k, v in d.items():
    print(k) # Bad (v unused)
for k, v in d.items():
    print(v) # Bad (k unused)
for _ in d.items(): pass    # Handled as tuple check usually, but our rule targets 2-element tuples
";
    let linter = run_linter(source, Config::default());
    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.rule_id == "CSP-P009")
        .collect();
    assert_eq!(
        findings.len(),
        5,
        "Should detect incorrect dict iterator usage"
    );
}

#[test]
fn test_global_usage_loop() {
    let source = r"
GLOBAL_CONST = 10
def func():
    LOCAL_CONST = 5
    for i in range(10):
        print(GLOBAL_CONST) # Bad
        print(LOCAL_CONST) # Good
";
    let linter = run_linter(source, Config::default());
    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.rule_id == "CSP-P010")
        .collect();
    assert_eq!(findings.len(), 1, "Should detect global usage in loop");
}

#[test]
fn test_memoryview_bytes_loop() {
    let source = r"
data = b'something'
for i in range(len(data)):
    chunk = data[0:i] # Bad

nums = [1, 2, 3]
for i in range(len(nums)):
    chunk = nums[0:i] # Good (not bytes)
";
    let linter = run_linter(source, Config::default());
    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.rule_id == "CSP-P011")
        .collect();
    assert_eq!(
        findings.len(),
        1,
        "Should detect looped slicing for bytes-like data"
    );
}

#[test]
fn test_use_tuple_over_list() {
    let source = r#"
CONST = [1, 2, "a", -3] # Bad
values = [1, 2]         # Good (not constant)
EMPTY = []              # Good (likely placeholder for mutation)
MIXED = [1, x]          # Good (non-literal)

class Box:
    CONST_CLASS = [1, 2] # Bad
    items = [1, 2]       # Good (not constant)

def func():
    CONST = [1, 2]       # Good (function scope)
"#;
    let linter = run_linter(source, Config::default());
    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.rule_id == "CSP-P012")
        .collect();
    assert_eq!(
        findings.len(),
        2,
        "Should suggest tuples for constant list literals"
    );
}

#[test]
fn test_comprehension_suggestion() {
    let source = r"
items = [1, 2, 3]
res = []
for x in items:
    res.append(x) # Bad

res2 = []
for x in items:
    if x > 1:
        res2.append(x) # Bad

res_set = set()
for x in items:
    res_set.add(x) # Bad

res_dict = {}
for x in items:
    res_dict[x] = x * 2 # Bad

res_set2 = set()
for x in items:
    if x > 1:
        res_set2.add(x) # Bad

res_dict2 = {}
for x in items:
    if x > 1:
        res_dict2[x] = x # Bad
";
    let linter = run_linter(source, Config::default());
    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.rule_id == "CSP-P013")
        .collect();
    assert_eq!(
        findings.len(),
        6,
        "Should detect loops that can be comprehensions"
    );
}

#[test]
fn test_pandas_chunksize_risk() {
    let source = r"
import pandas as pd
df = pd.read_csv('huge.csv') # Bad (CSP-P015)
df2 = pd.read_csv('huge.csv', chunksize=1000) # Good
df3 = pd.read_csv('huge.csv', nrows=100) # Good
df4 = pd.read_csv('huge.csv', iterator=True) # Good
";
    let linter = run_linter(source, Config::default());
    let findings: Vec<_> = linter
        .findings
        .iter()
        .filter(|f| f.rule_id == "CSP-P015")
        .collect();
    assert_eq!(
        findings.len(),
        1,
        "Should detect pandas.read_csv without chunksize"
    );
}
