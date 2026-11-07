use super::*;

// ── BoolOp (`a and b`) ──────────────────────────────────────────────────────
#[test]
fn fn_called_in_bool_op_not_unreachable() {
    let dir = mk_dir("cg_boolop_");
    write_py(
        &dir,
        "app.py",
        r"
def _check_a():
    return True

def _check_b():
    return True

def run():
    return _check_a() and _check_b()

run()
",
    );
    let result = app_analyzer().analyze(dir.path());
    let ur = unreachable_fns(&result);
    assert!(!ur.contains(&"_check_a".to_owned()), "got {ur:?}");
    assert!(!ur.contains(&"_check_b".to_owned()), "got {ur:?}");
}

// ── UnaryOp (`not f()`) ─────────────────────────────────────────────────────
#[test]
fn fn_called_in_unary_op_not_unreachable() {
    let dir = mk_dir("cg_unary_");
    write_py(
        &dir,
        "app.py",
        r"
def _flag():
    return True

def run():
    return not _flag()

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_flag".to_owned()), "got {ur:?}");
}

// ── Compare (`f() < g()`) ───────────────────────────────────────────────────
#[test]
fn fn_called_in_compare_not_unreachable() {
    let dir = mk_dir("cg_compare_");
    write_py(
        &dir,
        "app.py",
        r"
def _get_x():
    return 1

def _get_y():
    return 2

def run():
    return _get_x() < _get_y()

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_get_x".to_owned()), "got {ur:?}");
    assert!(!ur.contains(&"_get_y".to_owned()), "got {ur:?}");
}

// ── Tuple (`(f(), g())`) ────────────────────────────────────────────────────
#[test]
fn fn_called_inside_tuple_not_unreachable() {
    let dir = mk_dir("cg_tuple_");
    write_py(
        &dir,
        "app.py",
        r"
def _first():
    return 1

def _second():
    return 2

def run():
    return (_first(), _second())

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_first".to_owned()), "got {ur:?}");
    assert!(!ur.contains(&"_second".to_owned()), "got {ur:?}");
}

// ── Set (`{f()}`) ────────────────────────────────────────────────────────────
#[test]
fn fn_called_inside_set_literal_not_unreachable() {
    let dir = mk_dir("cg_set_");
    write_py(
        &dir,
        "app.py",
        r"
def _val():
    return 42

def run():
    return {_val()}

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_val".to_owned()), "got {ur:?}");
}

// ── Dict key containing a call (`{f(): 1}`) ─────────────────────────────────
#[test]
fn fn_called_as_dict_key_not_unreachable() {
    let dir = mk_dir("cg_dict_key_");
    write_py(
        &dir,
        "app.py",
        r"
def _make_key():
    return 'k'

def run():
    return {_make_key(): 1}

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_make_key".to_owned()), "got {ur:?}");
}

// ── ListComp element (`[f(x) for x in lst]`) ────────────────────────────────
#[test]
fn fn_called_in_listcomp_element_not_unreachable() {
    let dir = mk_dir("cg_listcomp_elt_");
    write_py(
        &dir,
        "app.py",
        r"
def _transform(x):
    return x * 2

def run():
    return [_transform(x) for x in range(5)]

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_transform".to_owned()), "got {ur:?}");
}

// ── ListComp iterable (`[x for x in f()]`) ──────────────────────────────────
#[test]
fn fn_called_as_listcomp_iterable_not_unreachable() {
    let dir = mk_dir("cg_listcomp_iter_");
    write_py(
        &dir,
        "app.py",
        r"
def _items():
    return [1, 2, 3]

def run():
    return [x for x in _items()]

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_items".to_owned()), "got {ur:?}");
}

// ── ListComp filter condition (`[x for x in lst if f(x)]`) ──────────────────
#[test]
fn fn_called_in_listcomp_condition_not_unreachable() {
    let dir = mk_dir("cg_listcomp_cond_");
    write_py(
        &dir,
        "app.py",
        r"
def _keep(x):
    return x > 0

def run():
    return [x for x in range(10) if _keep(x)]

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_keep".to_owned()), "got {ur:?}");
}

// ── DictComp (`{k: f(v) for k, v in items}`) ────────────────────────────────
#[test]
fn fn_called_in_dictcomp_not_unreachable() {
    let dir = mk_dir("cg_dictcomp_");
    write_py(
        &dir,
        "app.py",
        r"
def _process(v):
    return v + 1

def run():
    return {k: _process(v) for k, v in enumerate(range(3))}

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_process".to_owned()), "got {ur:?}");
}

// ── Generator expr (`sum(f(x) for x in lst)`) ───────────────────────────────
#[test]
fn fn_called_in_generator_expr_not_unreachable() {
    let dir = mk_dir("cg_genexpr_");
    write_py(
        &dir,
        "app.py",
        r"
def _score(x):
    return x * x

def run():
    return sum(_score(x) for x in range(5))

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_score".to_owned()), "got {ur:?}");
}

// ── Lambda body (`lambda: f()`) ─────────────────────────────────────────────
#[test]
fn fn_called_inside_lambda_body_not_unreachable() {
    let dir = mk_dir("cg_lambda_");
    write_py(
        &dir,
        "app.py",
        r"
def _compute():
    return 99

def run():
    fn = lambda: _compute()
    return fn()

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_compute".to_owned()), "got {ur:?}");
}

// ── Walrus operator (`x := f()`) ────────────────────────────────────────────
#[test]
fn fn_called_via_walrus_operator_not_unreachable() {
    let dir = mk_dir("cg_walrus_");
    write_py(
        &dir,
        "app.py",
        r"
def _get_value():
    return 5

def run():
    if (v := _get_value()) > 0:
        return v
    return 0

run()
",
    );
    let ur = unreachable_fns(&app_analyzer().analyze(dir.path()));
    assert!(!ur.contains(&"_get_value".to_owned()), "got {ur:?}");
}
