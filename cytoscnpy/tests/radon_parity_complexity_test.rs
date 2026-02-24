//! Comprehensive Radon complexity parity tests.
//! Ported from: `radon/tests/test_complexity_visitor.py`
//!
//! NOTE: `CytoScnPy`'s `analyze_complexity` only reports complexity for functions/methods,
//! not for module-level code. So we wrap test code in functions.

#![allow(clippy::unwrap_used)] // Tests use unwrap for clarity
#![allow(clippy::ignore_without_reason)] // Ignore reasons in comments

use cytoscnpy::complexity::analyze_complexity;
use std::path::PathBuf;

/// Helper to get function complexity by name
fn get_function_complexity(code: &str, name: &str) -> usize {
    let findings = analyze_complexity(code, &PathBuf::from("test.py"), false);
    findings
        .iter()
        .find(|f| f.name == name)
        .map_or(0, |f| f.complexity)
}

/// Helper to get function complexity with `no_assert` flag
fn get_function_complexity_no_assert(code: &str, name: &str) -> usize {
    let findings = analyze_complexity(code, &PathBuf::from("test.py"), true);
    findings
        .iter()
        .find(|f| f.name == name)
        .map_or(0, |f| f.complexity)
}

// =============================================================================
// SIMPLE BLOCKS - Basic control flow
// =============================================================================

#[test]
fn test_radon_if_simple() {
    let code = r"
def f():
    if a: pass
";
    // 1 (base) + 1 (if) = 2
    assert_eq!(get_function_complexity(code, "f"), 2);
}

#[test]
fn test_radon_if_else() {
    let code = r"
def f():
    if a: pass
    else: pass
";
    // else doesn't add complexity
    assert_eq!(get_function_complexity(code, "f"), 2);
}

#[test]
fn test_radon_if_elif() {
    let code = r"
def f():
    if a: pass
    elif b: pass
";
    // 1 (base) + 1 (if) + 1 (elif) = 3
    assert_eq!(get_function_complexity(code, "f"), 3);
}

#[test]
fn test_radon_if_elif_else() {
    let code = r"
def f():
    if a: pass
    elif b: pass
    else: pass
";
    assert_eq!(get_function_complexity(code, "f"), 3);
}

// =============================================================================
// BOOLEAN EXPRESSIONS - and/or in conditions add complexity
// =============================================================================

#[test]
fn test_radon_if_and() {
    let code = r"
def f():
    if a and b: pass
";
    // 1 (base) + 1 (if) + 1 (and) = 3
    assert_eq!(get_function_complexity(code, "f"), 3);
}

#[test]
fn test_radon_if_and_else() {
    let code = r"
def f():
    if a and b: pass
    else: pass
";
    assert_eq!(get_function_complexity(code, "f"), 3);
}

#[test]
fn test_radon_if_and_elif_and() {
    let code = r"
def f():
    if a and b: pass
    elif c and d: pass
    else: pass
";
    // 1 (base) + 1 (if) + 1 (and) + 1 (elif) + 1 (and) = 5
    assert_eq!(get_function_complexity(code, "f"), 5);
}

#[test]
fn test_radon_if_complex_boolean() {
    let code = r"
def f():
    if a and b or c and d: pass
    else: pass
";
    // 1 + 1 (if) + 1 (and) + 1 (or) + 1 (and) = 5
    assert_eq!(get_function_complexity(code, "f"), 5);
}

#[test]
fn test_radon_if_and_or() {
    let code = r"
def f():
    if a and b or c: pass
    else: pass
";
    // 1 + 1 (if) + 1 (and) + 1 (or) = 4
    assert_eq!(get_function_complexity(code, "f"), 4);
}

// =============================================================================
// LOOPS
// =============================================================================

#[test]
fn test_radon_for_simple() {
    let code = r"
def f():
    for x in range(10): print(x)
";
    // 1 (base) + 1 (for) = 2
    assert_eq!(get_function_complexity(code, "f"), 2);
}

#[test]
fn test_radon_for_else() {
    let code = r"
def f():
    for x in xrange(10): print(x)
    else: pass
";
    // for-else: 1 + 1 (for) + 1 (else) = 3
    assert_eq!(get_function_complexity(code, "f"), 3);
}

#[test]
fn test_radon_while_simple() {
    let code = r"
def f():
    while a < 4: pass
";
    // 1 (base) + 1 (while) = 2
    assert_eq!(get_function_complexity(code, "f"), 2);
}

#[test]
fn test_radon_while_else() {
    let code = r"
def f():
    while a < 4: pass
    else: pass
";
    // 1 + 1 (while) + 1 (else) = 3
    assert_eq!(get_function_complexity(code, "f"), 3);
}

#[test]
fn test_radon_while_and() {
    let code = r"
def f():
    while a < 4 and b < 42: pass
";
    // 1 + 1 (while) + 1 (and) = 3
    assert_eq!(get_function_complexity(code, "f"), 3);
}

#[test]
fn test_radon_while_complex() {
    let code = r"
def f():
    while a and b or c < 10: pass
    else: pass
";
    // 1 + 1 (while) + 1 (and) + 1 (or) + 1 (else) = 5
    assert_eq!(get_function_complexity(code, "f"), 5);
}

// =============================================================================
// WITH STATEMENTS - Don't count towards complexity per issue #123
// =============================================================================

#[test]
fn test_radon_with_no_complexity() {
    let code = r"
def f():
    with open('raw.py') as fobj: print(fobj.read())
";
    // with doesn't add complexity
    assert_eq!(get_function_complexity(code, "f"), 1);
}

// =============================================================================
// COMPREHENSIONS
// =============================================================================

#[test]
fn test_radon_list_comprehension() {
    let code = r"
def f():
    [i for i in range(4)]
";
    // 1 (base) + 1 (for in comprehension) = 2
    assert_eq!(get_function_complexity(code, "f"), 2);
}

#[test]
fn test_radon_list_comprehension_if() {
    let code = r"
def f():
    [i for i in range(4) if i&1]
";
    // 1 + 1 (for) + 1 (if) = 3
    assert_eq!(get_function_complexity(code, "f"), 3);
}

#[test]
fn test_radon_generator_expression() {
    let code = r"
def f():
    (i for i in range(4))
";
    // 1 + 1 (for) = 2
    assert_eq!(get_function_complexity(code, "f"), 2);
}

#[test]
fn test_radon_generator_expression_if() {
    let code = r"
def f():
    (i for i in range(4) if i&1)
";
    // 1 + 1 (for) + 1 (if) = 3
    assert_eq!(get_function_complexity(code, "f"), 3);
}

#[test]
fn test_radon_nested_generator() {
    let code = r"
def f():
    [i for i in range(42) if sum(k ** 2 for k in divisors(i)) & 1]
";
    // 1 + 1 (outer for) + 1 (outer if) + 1 (inner for) = 4
    assert_eq!(get_function_complexity(code, "f"), 4);
}

#[test]
fn test_radon_set_comprehension() {
    let code = r"
def f():
    {i for i in range(4)}
";
    assert_eq!(get_function_complexity(code, "f"), 2);
}

#[test]
fn test_radon_set_comprehension_if() {
    let code = r"
def f():
    {i for i in range(4) if i&1}
";
    assert_eq!(get_function_complexity(code, "f"), 3);
}

#[test]
fn test_radon_dict_comprehension() {
    let code = r"
def f():
    {i:i**4 for i in range(4)}
";
    assert_eq!(get_function_complexity(code, "f"), 2);
}

#[test]
fn test_radon_dict_comprehension_if() {
    let code = r"
def f():
    {i:i**4 for i in range(4) if i&1}
";
    assert_eq!(get_function_complexity(code, "f"), 3);
}

#[test]
fn test_radon_multiple_for_comprehension() {
    let code = r"
def f():
    sum(i for i in range(12) for z in range(i ** 2) if i * z & 1)
";
    // 1 + 1 (for i) + 1 (for z) + 1 (if) = 4
    assert_eq!(get_function_complexity(code, "f"), 4);
}

#[test]
fn test_radon_comprehension_complex_condition() {
    let code = r"
def f():
    sum(i for i in range(10) if i >= 2 and val and val2 or val3)
";
    // 1 + 1 (for) + 1 (if) + 1 (and) + 1 (and) + 1 (or) = 6
    assert_eq!(get_function_complexity(code, "f"), 6);
}

// =============================================================================
// TRY/EXCEPT
// =============================================================================

#[test]
fn test_radon_try_except() {
    let code = r"
def f():
    try: raise TypeError
    except TypeError: pass
";
    // 1 (base) + 1 (except) = 2
    assert_eq!(get_function_complexity(code, "f"), 2);
}

#[test]
fn test_radon_try_except_else() {
    let code = r"
def f():
    try: raise TypeError
    except TypeError: pass
    else: pass
";
    // 1 + 1 (except) + 1 (else) = 3
    assert_eq!(get_function_complexity(code, "f"), 3);
}

#[test]
fn test_radon_try_finally() {
    let code = r"
def f():
    try: raise TypeError
    finally: pass
";
    // finally doesn't add complexity
    assert_eq!(get_function_complexity(code, "f"), 1);
}

#[test]
fn test_radon_try_except_finally() {
    let code = r"
def f():
    try: raise TypeError
    except TypeError: pass
    finally: pass
";
    // 1 + 1 (except) = 2
    assert_eq!(get_function_complexity(code, "f"), 2);
}

#[test]
fn test_radon_try_except_else_finally() {
    let code = r"
def f():
    try: raise TypeError
    except TypeError: pass
    else: pass
    finally: pass
";
    // 1 + 1 (except) + 1 (else) = 3
    assert_eq!(get_function_complexity(code, "f"), 3);
}

// =============================================================================
// LAMBDA - Lambdas themselves don't add complexity per issue #68
// =============================================================================

#[test]
fn test_radon_lambda_simple() {
    let code = r"
def f():
    k = lambda a, b: k(b, a)
";
    // Lambda doesn't add to function's complexity
    assert_eq!(get_function_complexity(code, "f"), 1);
}

#[test]
fn test_radon_lambda_ternary() {
    let code = r"
def f():
    k = lambda a, b, c: c if a else b
";
    // Lambda doesn't add, but ternary does
    assert_eq!(get_function_complexity(code, "f"), 2);
}

// =============================================================================
// TERNARY EXPRESSIONS
// =============================================================================

#[test]
fn test_radon_ternary() {
    let code = r"
def f():
    v = a if b else c
";
    // 1 + 1 (ternary) = 2
    assert_eq!(get_function_complexity(code, "f"), 2);
}

#[test]
fn test_radon_ternary_with_generator() {
    let code = r"
def f():
    v = a if sum(i for i in xrange(c)) < 10 else c
";
    // 1 + 1 (ternary) + 1 (for in generator) = 3
    assert_eq!(get_function_complexity(code, "f"), 3);
}

// =============================================================================
// ASSERT STATEMENTS
// =============================================================================

#[test]
fn test_radon_assert_adds_complexity() {
    let code = r"
def f():
    assert i < 0
";
    // 1 + 1 (assert) = 2
    assert_eq!(get_function_complexity(code, "f"), 2);
}

#[test]
fn test_radon_assert_with_message() {
    let code = r#"
def f():
    assert i < 0, "Fail"
"#;
    assert_eq!(get_function_complexity(code, "f"), 2);
}

#[test]
fn test_radon_assert_no_assert_flag() {
    let code = r"
def f():
    assert i < 0
";
    // With no_assert=true, assert doesn't add complexity
    assert_eq!(get_function_complexity_no_assert(code, "f"), 1);
}

#[test]
fn test_radon_function_assert_no_assert() {
    let code = r"
def f():
   assert 10 > 20
";
    assert_eq!(get_function_complexity_no_assert(code, "f"), 1);
}

#[test]
fn test_radon_class_assert_no_assert() {
    let code = r"
class TestYo(object):
    def test_yo(self):
        assert self.n > 4
";
    assert_eq!(get_function_complexity_no_assert(code, "test_yo"), 1);
}

// =============================================================================
// FUNCTION TESTS - Complex functions
// =============================================================================

#[test]
fn test_radon_function_complex() {
    let code = r"
def f(a, b, c):
   if a and b == 4:
       return c ** c
   elif a and not c:
       return sum(i for i in range(41) if i&1)
   return a + b
";
    // 1 (base) + 1 (if) + 1 (and) + 1 (elif) + 1 (and) + 1 (for) + 1 (if) = 7
    assert_eq!(get_function_complexity(code, "f"), 7);
}

#[test]
fn test_radon_function_with_while() {
    let code = r"
def g(a, b):
   while a < b:
       b, a = a **2, b ** 2
   return b
";
    // 1 + 1 (while) = 2
    assert_eq!(get_function_complexity(code, "g"), 2);
}

#[test]
fn test_radon_function_with_nested_control() {
    let code = r"
def f(a, b):
   while a**b:
       a, b = b, a * (b - 1)
       if a and b:
           b = 0
       else:
           b = 1
   return sum(i for i in range(b))
";
    // 1 (base) + 1 (while) + 1 (if) + 1 (and) + 1 (for) = 5
    assert_eq!(get_function_complexity(code, "f"), 5);
}

// =============================================================================
// ASYNC/AWAIT (Python 3.5+)
// =============================================================================

#[test]
fn test_radon_async_function() {
    let code = r"
async def f(a, b):
   async with open('blabla.log', 'w') as fobj:
       async for i in range(100):
           fobj.write(str(i) + '\n')
";
    // 1 (base) + 1 (async for) = 2 (async with doesn't count)
    assert_eq!(get_function_complexity(code, "f"), 2);
}

// =============================================================================
// MATCH STATEMENTS (Python 3.10+)
// =============================================================================

#[test]
fn test_radon_match_single_case() {
    let code = r"
def f():
    match a:
        case 1: pass
";
    // 1 (base) + 1 (case) = 2
    assert_eq!(get_function_complexity(code, "f"), 2);
}

#[test]
fn test_radon_match_with_wildcard() {
    let code = r"
def f():
    match a:
        case 1: pass
        case _: pass
";
    // Wildcard case (_) doesn't add complexity
    assert_eq!(get_function_complexity(code, "f"), 2);
}

#[test]
fn test_radon_match_two_cases() {
    let code = r"
def f():
    match a:
        case 1: pass
        case 2: pass
";
    // 1 + 1 + 1 = 3
    assert_eq!(get_function_complexity(code, "f"), 3);
}

#[test]
fn test_radon_match_two_cases_wildcard() {
    let code = r"
def f():
    match a:
        case 1: pass
        case 2: pass
        case _: pass
";
    // wildcard doesn't add
    assert_eq!(get_function_complexity(code, "f"), 3);
}

// =============================================================================
// CLOSURES / NESTED FUNCTIONS
// =============================================================================

#[test]
fn test_radon_closures() {
    let code = r"
def f(n):
    def g(l):
        return l ** 4
    def h(i):
        return i ** 5 + 1 if i & 1 else 2
    return sum(g(u + 4) / float(h(u)) for u in range(2, n))
";
    // f: 1 (base) + 1 (sum generator for) = 2
    // g: 1 (base) = 1
    // h: 1 (base) + 1 (ternary) = 2
    let findings = analyze_complexity(code, &PathBuf::from("test.py"), false);

    let g = findings.iter().find(|x| x.name == "g");
    let h = findings.iter().find(|x| x.name == "h");

    assert!(g.is_some(), "g not found");
    assert!(h.is_some(), "h not found");

    assert_eq!(g.unwrap().complexity, 1);
    assert_eq!(h.unwrap().complexity, 2);
}

#[test]
fn test_radon_memoize_pattern() {
    let code = r"
def memoize(func):
    cache = {}
    def aux(*args, **kwargs):
        key = (args, kwargs)
        if key in cache:
            return cache[key]
        cache[key] = res = func(*args, **kwargs)
        return res
    return aux
";
    // aux: 1 base + 1 if = 2
    assert_eq!(get_function_complexity(code, "aux"), 2);
}

// =============================================================================
// CLASS TESTS
// =============================================================================

#[test]
fn test_radon_class_methods() {
    let code = r"
class A(object):

    def m(self, a, b):
        if not a or b:
            return b - 1
        try:
            return a / b
        except ZeroDivisionError:
            return a

    def n(self, k):
        while self.m(k) < k:
            k -= self.m(k ** 2 - min(self.m(j) for j in range(k ** 4)))
        return k
";
    // m: 1 (base) + 1 (if) + 1 (or) + 1 (except) = 4
    assert_eq!(get_function_complexity(code, "m"), 4);

    // n: 1 (base) + 1 (while) + 1 (for) = 3
    assert_eq!(get_function_complexity(code, "n"), 3);
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[test]
fn test_radon_empty_function() {
    let code = r"
def f():
    pass
";
    assert_eq!(get_function_complexity(code, "f"), 1);
}

#[test]
fn test_radon_nested_if() {
    let code = r"
def f():
    if a:
        if b:
            if c:
                pass
";
    // 1 + 1 + 1 + 1 = 4
    assert_eq!(get_function_complexity(code, "f"), 4);
}

#[test]
fn test_radon_multiple_except() {
    let code = r"
def f():
    try:
        pass
    except ValueError:
        pass
    except TypeError:
        pass
    except:
        pass
";
    // 1 + 3 (except handlers) = 4
    assert_eq!(get_function_complexity(code, "f"), 4);
}

// =============================================================================
// TODO: MODULE-LEVEL COMPLEXITY TESTS
// These tests will FAIL until module-level complexity reporting is implemented.
// Radon reports complexity for code at the module level (outside functions).
// CytoScnPy currently only reports complexity for functions/methods.
// =============================================================================

/// Helper to get total module-level complexity (sum of all findings)
fn get_module_complexity(code: &str) -> usize {
    let findings = analyze_complexity(code, &PathBuf::from("test.py"), false);
    findings.iter().map(|f| f.complexity).sum()
}

#[test]
fn test_radon_module_level_if() {
    // Module-level if statement (not inside any function)
    let code = r"
if a: pass
";
    // Expected: 1 (base) + 1 (if) = 2
    // Currently fails because CytoScnPy doesn't report module-level complexity
    assert_eq!(get_module_complexity(code), 2);
}

#[test]
fn test_radon_module_level_if_elif() {
    let code = r"
if a: pass
elif b: pass
else: pass
";
    // Expected: 3 (if + elif)
    assert_eq!(get_module_complexity(code), 3);
}

#[test]
fn test_radon_module_level_for() {
    let code = r"
for x in range(10): print(x)
";
    // Expected: 2 (base + for)
    assert_eq!(get_module_complexity(code), 2);
}

#[test]
fn test_radon_module_level_while() {
    let code = r"
while a < 4: pass
";
    // Expected: 2 (base + while)
    assert_eq!(get_module_complexity(code), 2);
}

#[test]
fn test_radon_module_level_try_except() {
    let code = r"
try: raise TypeError
except TypeError: pass
";
    // Expected: 2 (base + except)
    assert_eq!(get_module_complexity(code), 2);
}

#[test]
fn test_radon_module_level_mixed_with_function() {
    // Both module-level code AND a function
    let code = r"
if config.debug:
    setup_logging()
elif config.verbose:
    setup_verbose()

def process():
    if x:
        return 1
    return 0
";
    // Expected: Module level = 3 (if + elif), Function = 2 (if)
    // Total = 5 (if Radon includes both)
    // CytoScnPy currently only reports the function's complexity = 2
    let findings = analyze_complexity(code, &PathBuf::from("test.py"), false);

    // Check we have the function
    let process = findings.iter().find(|f| f.name == "process");
    assert!(process.is_some(), "process function should be found");
    assert_eq!(process.unwrap().complexity, 2);

    // Check we also have module-level complexity
    // This will FAIL until module-level complexity is implemented
    let module_level = findings
        .iter()
        .find(|f| f.name == "<module>" || f.name.is_empty());
    assert!(
        module_level.is_some(),
        "Module-level complexity should be reported"
    );
    assert_eq!(module_level.unwrap().complexity, 3);
}

#[test]
fn test_radon_module_level_comprehension() {
    let code = r"
[i for i in range(4) if i&1]
";
    // Expected: 3 (base + for + if in comprehension)
    assert_eq!(get_module_complexity(code), 3);
}

#[test]
fn test_radon_module_level_boolean_expr() {
    let code = r"
if a and b or c: pass
";
    // Expected: 4 (base + if + and + or)
    assert_eq!(get_module_complexity(code), 4);
}
