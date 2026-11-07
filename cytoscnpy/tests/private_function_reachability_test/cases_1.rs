use super::*;

// ---------------------------------------------------------------------------
// 1. Private function called only inside an `AugAssign` (x += _helper())
//    The call-graph builder previously skipped AugAssign bodies, causing the
//    callee to appear unreachable even though it was clearly referenced.
// ---------------------------------------------------------------------------
#[test]
fn private_fn_called_in_augassign_is_not_unreachable() {
    let code = r"
def _helper():
    return 42

def run():
    total = 0
    total += _helper()
    return total

run()
";
    let result = app_analyzer().analyze_code(code, std::path::Path::new("app.py"));

    let unreachable_names: Vec<_> = result
        .unused_functions
        .iter()
        .filter(|d| d.is_unreachable)
        .map(|d| d.simple_name.as_str())
        .collect();

    assert!(
        !unreachable_names.contains(&"_helper"),
        "_helper is called inside AugAssign and must NOT be unreachable; got {unreachable_names:?}"
    );
}

// ---------------------------------------------------------------------------
// 2. Private function used as a keyword argument value.
//    call_graph previously did not traverse keyword arg values.
// ---------------------------------------------------------------------------
#[test]
fn private_fn_used_as_keyword_arg_is_not_unreachable() {
    let code = r"
def _compute():
    return 7

def build(**kwargs):
    pass

def main():
    build(value=_compute())

main()
";
    let result = app_analyzer().analyze_code(code, std::path::Path::new("app.py"));

    let unreachable_names: Vec<_> = result
        .unused_functions
        .iter()
        .filter(|d| d.is_unreachable)
        .map(|d| d.simple_name.as_str())
        .collect();

    assert!(
        !unreachable_names.contains(&"_compute"),
        "_compute used as kwarg must NOT be unreachable; got {unreachable_names:?}"
    );
}

// ---------------------------------------------------------------------------
// 3. Private method called normally via `self.` — baseline sanity check.
// ---------------------------------------------------------------------------
#[test]
fn private_method_called_via_self_is_not_unreachable() {
    let code = r"
class Processor:
    def _validate(self, x):
        return x > 0

    def process(self, x):
        if self._validate(x):
            return x
        return 0

Processor().process(5)
";
    let result = app_analyzer().analyze_code(code, std::path::Path::new("app.py"));

    let all_names: Vec<_> = result
        .unused_functions
        .iter()
        .chain(result.unused_methods.iter())
        .map(|d| d.simple_name.as_str())
        .collect();

    assert!(
        !all_names.contains(&"_validate"),
        "_validate is called via self and must not be reported; got {all_names:?}"
    );
}

// ---------------------------------------------------------------------------
// 4. Pragma-suppressed function that is also structurally unreachable.
//    confidence == 0 must prevent reporting regardless of is_unreachable.
// ---------------------------------------------------------------------------
#[test]
fn pragma_suppressed_private_fn_never_reported_even_if_unreachable() {
    let dir = mk_tempdir("pragma_reach_");
    let path = dir.path().join("module.py");
    let mut f = File::create(&path).unwrap();
    writeln!(
        f,
        r"
def _suppressed():  # pragma: no cytoscnpy
    pass

def main():
    pass

main()
"
    )
    .unwrap();

    let result = app_analyzer().analyze(dir.path());

    let reported: Vec<_> = result
        .unused_functions
        .iter()
        .map(|d| d.simple_name.as_str())
        .collect();

    assert!(
        !reported.contains(&"_suppressed"),
        "pragma-suppressed _suppressed must NEVER be reported; got {reported:?}"
    );
}

// ---------------------------------------------------------------------------
// 5. Truly dead private function — never referenced anywhere.
//    It should still be surfaced as unreachable dead code.
//    Uses directory analysis (aggregate_results) because call-graph reachability
//    only runs in that mode.
// ---------------------------------------------------------------------------
#[test]
fn truly_dead_private_fn_is_reported_as_unreachable() {
    let dir = mk_tempdir("dead_priv_");
    let path = dir.path().join("app.py");
    let mut f = File::create(&path).unwrap();
    writeln!(
        f,
        r"
def _dead():
    return 999

def main():
    pass

main()
"
    )
    .unwrap();

    let result = app_analyzer().analyze(dir.path());

    let unreachable: Vec<_> = result
        .unused_functions
        .iter()
        .filter(|d| d.is_unreachable)
        .map(|d| d.simple_name.as_str())
        .collect();

    assert!(
        unreachable.contains(&"_dead"),
        "truly dead _dead must be reported as unreachable; got {unreachable:?}"
    );
}

// ---------------------------------------------------------------------------
// 6. Private method of an unreachable class promoted even though
//    its confidence < threshold (Fix D in promote_methods_from_unused_classes).
// ---------------------------------------------------------------------------
#[test]
fn private_method_of_unreachable_class_promoted_regardless_of_confidence() {
    let dir = mk_tempdir("dead_cls_priv_");
    let path = dir.path().join("module.py");
    let mut f = File::create(&path).unwrap();
    writeln!(
        f,
        r"
class DeadClass:
    def _internal(self):
        return 1

    def also_dead(self):
        return self._internal()

def main():
    pass

main()
"
    )
    .unwrap();

    let result = app_analyzer().analyze(dir.path());

    // DeadClass should be an unreachable class
    let dead_classes: Vec<_> = result
        .unused_classes
        .iter()
        .filter(|d| d.is_unreachable)
        .map(|d| d.simple_name.as_str())
        .collect();
    assert!(
        dead_classes.contains(&"DeadClass"),
        "DeadClass should be reported as unreachable; got {dead_classes:?}"
    );

    // _internal is private (confidence 20) but belongs to unreachable class —
    // promote_methods_from_unused_classes must surface it.
    let dead_methods: Vec<_> = result
        .unused_methods
        .iter()
        .map(|d| d.simple_name.as_str())
        .collect();
    assert!(
        dead_methods.contains(&"_internal"),
        "_internal of unreachable DeadClass must be promoted to unused_methods; got {dead_methods:?}"
    );
}

// ---------------------------------------------------------------------------
// 7. Cross-class private method false-negative scenario.
//    ClassA._process is reachable; ClassB._process is dead code.
//    Loose attr matching (`._process`) must not save ClassB._process from
//    being reported when ClassB itself is unreachable.
// ---------------------------------------------------------------------------
#[test]
fn dead_class_private_method_not_shielded_by_cross_class_attr_ref() {
    let dir = mk_tempdir("cross_cls_priv_");
    let path = dir.path().join("module.py");
    let mut f = File::create(&path).unwrap();
    writeln!(
        f,
        r"
class ActiveClass:
    def _process(self):
        return 1

    def run(self):
        return self._process()

class DeadClass:
    def _process(self):  # same name, dead class
        return 2

    def run(self):
        return self._process()

ActiveClass().run()
"
    )
    .unwrap();

    let result = app_analyzer().analyze(dir.path());

    // DeadClass must be unreachable
    let dead_classes: Vec<_> = result
        .unused_classes
        .iter()
        .filter(|d| d.is_unreachable)
        .map(|d| d.simple_name.as_str())
        .collect();
    assert!(
        dead_classes.contains(&"DeadClass"),
        "DeadClass should be unreachable; got {dead_classes:?}"
    );

    // DeadClass._process must be promoted as a dead method
    let dead_methods: Vec<_> = result
        .unused_methods
        .iter()
        .map(|d| d.full_name.as_str())
        .collect();

    // Note: DeadClass._process has confidence 20 (private), so promotion only
    // succeeds with the unreachable-class bypass added in Fix D.
    // The cross-class `._process` attr ref must NOT shield it from promotion.
    let promoted = dead_methods
        .iter()
        .any(|n| n.contains("DeadClass") && n.contains("_process"));
    assert!(
        promoted,
        "DeadClass._process (private, dead class) must be promoted; methods={dead_methods:?}"
    );
}
