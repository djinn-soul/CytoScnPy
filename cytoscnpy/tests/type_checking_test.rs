//! Tests for type checking import resolution.

use cytoscnpy::analyzer::CytoScnPy;
use std::path::PathBuf;

#[test]
fn test_type_checking_imports() {
    let test_file_path = PathBuf::from("type_checking_example.py");
    let cytoscnpy = CytoScnPy::default().with_confidence(100).with_tests(false);

    let code = r#"
import typing
import typing_extensions
from typing import TYPE_CHECKING
from typing_extensions import TYPE_CHECKING as TC

if TYPE_CHECKING:
    import unused_module
    from database import Database

if typing.TYPE_CHECKING:
    from database import Database

if typing_extensions.TYPE_CHECKING:
    from config import Config

if TC:
    from service import Service

def process(user: "User", service: "Service", db: "Database", config: "Config"):
    pass

def helper(h: Helper):
    pass
"#;

    // Analyze the code
    let report = cytoscnpy.analyze_code(code, &test_file_path);

    // Check results
    // 1. "unused_module" SHOULD be reported as unused (it's in TYPE_CHECKING but not used)
    let found_unused_module = report
        .unused_imports
        .iter()
        .any(|i| i.simple_name == "unused_module");
    assert!(
        found_unused_module,
        "Should detect 'unused_module' as unused even inside TYPE_CHECKING"
    );

    // 2. "User" SHOULD NOT be reported (used in string annotation "User")
    let found_user = report
        .unused_imports
        .iter()
        .any(|i| i.simple_name == "User");
    assert!(
        !found_user,
        "Should NOT report 'User' as unused (used in string annotation)"
    );

    // 3. "Service" SHOULD NOT be reported (used in string annotation "Service", guarded by TC alias)
    let found_service = report
        .unused_imports
        .iter()
        .any(|i| i.simple_name == "Service");
    assert!(
        !found_service,
        "Should NOT report 'Service' as unused (guarded by TC alias)"
    );

    // 4. "Database" SHOULD NOT be reported (used in string annotation "Database", guarded by typing.TYPE_CHECKING)
    let found_db = report
        .unused_imports
        .iter()
        .any(|i| i.simple_name == "Database");
    assert!(
        !found_db,
        "Should NOT report 'Database' as unused (guarded by typing.TYPE_CHECKING)"
    );

    // 5. "Config" SHOULD NOT be reported (used in string annotation "Config", guarded by typing_extensions.TYPE_CHECKING)
    let found_config = report
        .unused_imports
        .iter()
        .any(|i| i.simple_name == "Config");
    assert!(
        !found_config,
        "Should NOT report 'Config' as unused (guarded by typing_extensions.TYPE_CHECKING)"
    );

    // 6. "Helper" SHOULD NOT be reported (used in normal annotation)
    let found_helper = report
        .unused_imports
        .iter()
        .any(|i| i.simple_name == "Helper");
    assert!(
        !found_helper,
        "Should NOT report 'Helper' as unused (used in normal annotation)"
    );
}

fn unused_import_names(code: &str) -> Vec<String> {
    let cytoscnpy = CytoScnPy::default().with_confidence(100).with_tests(false);
    cytoscnpy
        .analyze_code(code, &PathBuf::from("annotations_example.py"))
        .unused_imports
        .into_iter()
        .map(|i| i.simple_name)
        .collect()
}

#[test]
fn test_string_annotations_with_spaces_mark_imports_used() {
    let code = r#"
from typing import TYPE_CHECKING, Any, Optional, TypeAlias, TypeVar, cast

if TYPE_CHECKING:
    from collections import OrderedDict
    from decimal import Decimal
    from fractions import Fraction
    from numpy import ndarray
    from pathlib import Path

Alias: TypeAlias = "dict[str, Path]"
T = TypeVar("T", bound="Optional[Fraction]")

class Cfg:
    field: "OrderedDict[str, int] | None" = None

def process(x: "ndarray | None") -> "dict[str, Any]":
    return cast("Optional[Decimal]", x)
"#;
    let unused = unused_import_names(code);
    assert!(unused.is_empty(), "unexpected unused imports: {unused:?}");
}

#[test]
fn test_invalid_string_annotation_does_not_mark_imports_used() {
    let code = r#"
from typing import Any

def process(x: "not a valid type") -> None:
    pass

note = "dict[str, Any]"
"#;
    assert_eq!(unused_import_names(code), vec!["Any".to_owned()]);
}

#[test]
fn test_pep695_type_param_bounds_mark_imports_used() {
    let code = r"
from typing import Any
from numbers import Number

def first[T: Any](items: list[T]) -> T:
    return items[0]

class Box[N: Number]:
    pass
";
    let unused = unused_import_names(code);
    assert!(unused.is_empty(), "unexpected unused imports: {unused:?}");
}

#[test]
fn test_dotted_module_imports_used_via_attribute_chain() {
    let code = r#"
import collections.abc
import os.path
import xml.etree.ElementTree

def check(x: collections.abc.Iterable) -> str:
    return os.path.join("a", xml.etree.ElementTree.tostring(x))
"#;
    let unused = unused_import_names(code);
    assert!(unused.is_empty(), "unexpected unused imports: {unused:?}");
}

#[test]
fn test_dotted_module_import_used_when_parent_name_used() {
    // `import os.path` binds `os`, so using `os` uses it (as in pyflakes/ruff).
    let code = r"
import os
import os.path
import importlib.util

print(os.getcwd(), importlib.import_module)
";
    let unused = unused_import_names(code);
    assert!(unused.is_empty(), "unexpected unused imports: {unused:?}");
}

#[test]
fn test_dotted_module_import_unused_when_never_used() {
    let unused = unused_import_names("import os.path\n");
    assert_eq!(unused.len(), 1, "unexpected unused imports: {unused:?}");
}

#[test]
fn test_strings_in_runtime_generic_types_mark_imports_used() {
    let code = r#"
import os
import typing as t
from typing import Union
from decimal import Decimal
from fractions import Fraction

PathLike = Union[str, "os.PathLike[str]"]
Number = t.Optional["Decimal | None"]
Items = list["Fraction"]
"#;
    let unused = unused_import_names(code);
    assert!(unused.is_empty(), "unexpected unused imports: {unused:?}");
}

#[test]
fn test_functional_typed_dict_and_named_tuple_mark_imports_used() {
    let code = r#"
from typing import NamedTuple, TypedDict
from decimal import Decimal
from fractions import Fraction

Point = TypedDict("Point", {"x": "Decimal | None"})
Pair = NamedTuple("Pair", [("a", "list[Fraction]")])
"#;
    let unused = unused_import_names(code);
    assert!(unused.is_empty(), "unexpected unused imports: {unused:?}");
}

#[test]
fn test_redundant_alias_is_explicit_reexport() {
    let code = r"
import os as os
from collections import OrderedDict as OrderedDict
from decimal import Decimal as D
";
    assert_eq!(unused_import_names(code), vec!["D".to_owned()]);
}

#[test]
fn test_pyflakes_noqa_codes_suppress_unused_imports() {
    let code = r"
from typing import Any  # noqa: F401
from typing import (  # noqa: F401
    List,
    Set,
)
from typing import (  # noqa
    Dict,
)
from typing import (  # noqa: E501
    Tuple,
)
";
    assert_eq!(unused_import_names(code), vec!["Tuple".to_owned()]);
}
