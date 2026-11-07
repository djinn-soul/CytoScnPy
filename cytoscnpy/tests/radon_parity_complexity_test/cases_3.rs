use super::*;

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
