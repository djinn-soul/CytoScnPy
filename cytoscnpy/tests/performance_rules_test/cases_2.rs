use super::*;

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
