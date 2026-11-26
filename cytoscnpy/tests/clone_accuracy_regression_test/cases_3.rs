use super::*;

// --- Recall guards: the false-positive heuristics above must not hide
// genuine clones. Each pair is a real copy-paste that differs only in names.

const SHARED_METHOD: &str = "    def describe(self, items):\n        rows = []\n        for item in items:\n            if item.ok:\n                rows.append(item.value * 2)\n        return sum(rows)\n";

fn class_pair(
    header: &str,
    left: &str,
    right: &str,
    body_left: &str,
    body_right: &str,
) -> (String, String) {
    (
        format!("{header}class {left}{body_left}{SHARED_METHOD}"),
        format!("{header}class {right}{body_right}{SHARED_METHOD}"),
    )
}

#[test]
fn concrete_abc_subclasses_remain_clones() {
    let (a, b) = class_pair(
        "from abc import ABC\n\n",
        "RepoA",
        "RepoB",
        "(ABC):\n",
        "(ABC):\n",
    );
    assert!(
        detects_named_pair(&a, &b, "RepoA", "RepoB"),
        "ABC subclasses with real method bodies must still be detected"
    );
}

#[test]
fn asyncio_protocol_implementations_remain_clones() {
    let (a, b) = class_pair(
        "import asyncio\n\n",
        "ServerA",
        "ServerB",
        "(asyncio.Protocol):\n",
        "(asyncio.Protocol):\n",
    );
    assert!(
        detects_named_pair(&a, &b, "ServerA", "ServerB"),
        "concrete asyncio.Protocol subclasses must still be detected"
    );
}

#[test]
fn exceptions_with_logic_in_init_remain_clones() {
    let body = "(Exception):\n    def __init__(self, fields):\n        problems = []\n        for name, value in fields.items():\n            if value is None:\n                problems.append(name)\n        super().__init__(\", \".join(problems))\n        self.problems = problems\n";
    let a = format!("class ValidationError{body}");
    let b = format!("class SchemaError{body}");
    assert!(
        detects_named_pair(&a, &b, "ValidationError", "SchemaError"),
        "exception classes with non-trivial __init__ logic must still be detected"
    );
}

#[test]
fn error_named_classes_without_exception_base_remain_clones() {
    let body = ":\n    def __init__(self, line, column):\n        self.line = line\n        self.column = column\n";
    let (a, b) = class_pair("", "ParseError", "LexError", body, body);
    assert!(
        detects_named_pair(&a, &b, "ParseError", "LexError"),
        "a class is not an exception just because its name ends in Error"
    );
}

#[test]
fn single_string_subscript_keys_still_normalise() {
    let body = "(cfg, x):\n    value = cfg[\"{key}\"]\n    scaled = value * 2\n    if scaled > x:\n        scaled = x\n    result = scaled + x\n    return result\n";
    let a = format!("def host_url{}", body.replace("{key}", "host"));
    let b = format!("def port_url{}", body.replace("{key}", "port"));
    assert!(
        detects_named_pair(&a, &b, "host_url", "port_url"),
        "string subscripts outside record access must keep literal normalisation"
    );
}

#[test]
fn keras_model_is_not_a_schema_base() {
    let (a, b) = class_pair(
        "import keras\n\n",
        "NetA",
        "NetB",
        "(keras.Model):\n    depth = 4\n    width = 32\n    dropout = 0.1\n",
        "(keras.Model):\n    layers = 4\n    units = 32\n    rate = 0.1\n",
    );
    assert!(
        detects_named_pair(&a, &b, "NetA", "NetB"),
        "keras.Model is not a schema base; constant renames remain Type-2 clones"
    );
}

#[test]
fn project_local_base_is_not_a_schema_base() {
    let (a, b) = class_pair(
        "from app.core import Base\n\n",
        "JobA",
        "JobB",
        "(Base):\n    retries = 3\n    delay = 5\n    queue = 'a'\n",
        "(Base):\n    attempts = 3\n    pause = 5\n    channel = 'b'\n",
    );
    assert!(
        detects_named_pair(&a, &b, "JobA", "JobB"),
        "a project-local `Base` is not a schema base"
    );
}

#[test]
fn unrelated_define_decorator_is_not_a_schema_marker() {
    let (a, b) = class_pair(
        "from registry import define\n\n@define\n",
        "HandlerA",
        "HandlerB",
        ":\n    name: str = 'a'\n    retries: int = 3\n",
        ":\n    label: str = 'b'\n    attempts: int = 3\n",
    );
    assert!(
        detects_named_pair(&a, &b, "HandlerA", "HandlerB"),
        "only attrs' define/frozen/mutable/s mark a schema class"
    );
}

#[test]
fn feature_flag_subclasses_are_not_enums() {
    let (a, b) = class_pair(
        "from app.flags import FeatureFlag\n\n",
        "Toggle",
        "Switch",
        "(FeatureFlag):\n    NAME = 'beta'\n    ENABLED = True\n    ROLLOUT = 50\n",
        "(FeatureFlag):\n    KEY = 'alpha'\n    ACTIVE = False\n    PERCENT = 10\n",
    );
    assert!(
        detects_named_pair(&a, &b, "Toggle", "Switch"),
        "FeatureFlag is not an enum base; member renames remain Type-2 clones"
    );
}
