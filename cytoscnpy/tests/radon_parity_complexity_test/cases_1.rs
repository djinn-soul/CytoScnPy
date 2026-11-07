use super::*;

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
