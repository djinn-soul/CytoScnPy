use super::*;

// ---------------------------------------------------------------------------
// 8. Private function called inside an annotated assignment (AnnAssign).
// ---------------------------------------------------------------------------
#[test]
fn private_fn_called_in_ann_assign_is_not_unreachable() {
    let code = r"
def _build_config():
    return {'key': 'value'}

def setup():
    config: dict = _build_config()
    return config

setup()
";
    let result = app_analyzer().analyze_code(code, std::path::Path::new("app.py"));

    let unreachable_names: Vec<_> = result
        .unused_functions
        .iter()
        .filter(|d| d.is_unreachable)
        .map(|d| d.simple_name.as_str())
        .collect();

    assert!(
        !unreachable_names.contains(&"_build_config"),
        "_build_config called in AnnAssign must NOT be unreachable; got {unreachable_names:?}"
    );
}

// ---------------------------------------------------------------------------
// 9. Private function called only inside an `assert` statement.
// ---------------------------------------------------------------------------
#[test]
fn private_fn_called_in_assert_is_not_unreachable() {
    let code = r"
def _check(x):
    return x > 0

def run(x):
    assert _check(x), 'invalid'
    return x

run(1)
";
    let result = app_analyzer().analyze_code(code, std::path::Path::new("app.py"));

    let unreachable_names: Vec<_> = result
        .unused_functions
        .iter()
        .filter(|d| d.is_unreachable)
        .map(|d| d.simple_name.as_str())
        .collect();

    assert!(
        !unreachable_names.contains(&"_check"),
        "_check called in assert must NOT be unreachable; got {unreachable_names:?}"
    );
}
