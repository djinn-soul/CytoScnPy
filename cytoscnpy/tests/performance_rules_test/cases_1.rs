use super::*;

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
