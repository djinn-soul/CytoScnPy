use super::*;

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
