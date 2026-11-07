use super::*;

#[test]
fn test_lambda_functions() {
    let code = r"
double = lambda x: x * 2
";
    visit_code!(code, visitor);

    let _ref_names: HashSet<String> = visitor.references.iter().map(|(n, _)| n.clone()).collect();
}

#[test]
fn test_attribute_access_chains() {
    let code = r#"
result = text.upper().replace(" ", "_")
"#;
    visit_code!(code, visitor);

    let ref_names: HashSet<String> = visitor.references.iter().map(|(n, _)| n.clone()).collect();

    assert!(ref_names.contains(".upper"));
    assert!(ref_names.contains(".replace"));
}

#[test]
fn test_star_imports() {
    let code = r"
from os import *
";
    visit_code!(code, visitor);

    // Star imports must NOT create a bogus `*` definition — they are recorded
    // in `star_imports` for cross-file resolution during aggregation.
    let star_defs: Vec<_> = visitor
        .definitions
        .iter()
        .filter(|d| d.simple_name == "*")
        .collect();
    assert!(
        star_defs.is_empty(),
        "`from x import *` must not create a `*` definition"
    );
    assert!(
        visitor.star_imports.contains(&"os".to_owned()),
        "star_imports should record the source module"
    );
}

#[test]
fn test_try_except_handling() {
    let code = r"
def process_file():
    try:
        result = open_file()
    except FileNotFoundError:
        handle_missing()
    except ValueError as e:
        log_error(e)
    except (TypeError, KeyError):
        handle_type_error()
    finally:
        cleanup()
";
    visit_code!(code, visitor);

    // Check that function is defined
    let functions: Vec<_> = visitor
        .definitions
        .iter()
        .filter(|d| d.def_type == "function")
        .collect();
    assert_eq!(functions.len(), 1);
    assert_eq!(functions[0].simple_name, "process_file");

    // Check that references in try/except bodies are captured
    let ref_names: HashSet<String> = visitor.references.iter().map(|(n, _)| n.clone()).collect();

    // References from try body
    assert!(ref_names.contains("open_file") || ref_names.contains("test.open_file"));

    // References from except handlers
    assert!(ref_names.contains("handle_missing") || ref_names.contains("test.handle_missing"));
    assert!(ref_names.contains("log_error") || ref_names.contains("test.log_error"));
    assert!(
        ref_names.contains("handle_type_error") || ref_names.contains("test.handle_type_error")
    );

    // References from finally block
    assert!(ref_names.contains("cleanup") || ref_names.contains("test.cleanup"));

    // Exception type references
    assert!(
        ref_names.contains("FileNotFoundError") || ref_names.contains("test.FileNotFoundError")
    );
    assert!(ref_names.contains("ValueError") || ref_names.contains("test.ValueError"));
    assert!(ref_names.contains("TypeError") || ref_names.contains("test.TypeError"));
    assert!(ref_names.contains("KeyError") || ref_names.contains("test.KeyError"));
}

#[test]
fn test_future_imports_ignored() {
    // __future__ imports are compiler directives, not real imports
    // They should NOT be added as definitions to avoid false "unused import" positives
    let code = r"
from __future__ import annotations
from __future__ import division, print_function

import os
from pathlib import Path
";
    visit_code!(code, visitor);

    // Only real imports should be added as definitions
    let import_defs: Vec<_> = visitor
        .definitions
        .iter()
        .filter(|d| d.def_type == "import")
        .collect();

    let import_names: HashSet<String> = import_defs.iter().map(|d| d.simple_name.clone()).collect();

    // __future__ imports should NOT be in definitions
    assert!(
        !import_names.contains("annotations"),
        "annotations from __future__ should not be tracked as import"
    );
    assert!(
        !import_names.contains("division"),
        "division from __future__ should not be tracked as import"
    );
    assert!(
        !import_names.contains("print_function"),
        "print_function from __future__ should not be tracked as import"
    );

    // Regular imports SHOULD be tracked
    assert!(
        import_names.contains("os"),
        "regular import 'os' should be tracked"
    );
    assert!(
        import_names.contains("Path"),
        "regular import 'Path' should be tracked"
    );

    // Total should be 2 (os, Path) - not 5
    assert_eq!(
        import_defs.len(),
        2,
        "Only 2 real imports should be tracked, not __future__ imports"
    );
}

#[test]
fn test_alias_resolution() {
    // When using an aliased import, both the alias and original name should be tracked
    let code = r"
import pandas as pd
from os.path import join as path_join

df = pd.DataFrame()
result = path_join('a', 'b')
";
    visit_code!(code, visitor);

    let ref_names: HashSet<String> = visitor.references.iter().map(|(n, _)| n.clone()).collect();

    // Using 'pd' should add reference to 'pandas' (the original)
    assert!(
        ref_names.contains("pandas"),
        "Using alias 'pd' should resolve to original 'pandas'"
    );

    // Using 'path_join' should add reference to 'os.path.join' (the original qualified name)
    assert!(
        ref_names.contains("os.path.join"),
        "Using alias 'path_join' should resolve to original 'os.path.join'"
    );

    // Should also add simple name 'join' for qualified imports
    assert!(
        ref_names.contains("join"),
        "Using qualified alias should also add simple name 'join'"
    );
}

#[test]
fn test_pep695_type_alias_statement_tracking() {
    let code = r"
type UserId = int

def use_alias(x: UserId) -> UserId:
    return x
";
    visit_code!(code, visitor);

    let alias_def = visitor
        .definitions
        .iter()
        .find(|d| d.simple_name == "UserId" && d.def_type == "variable");
    assert!(alias_def.is_some(), "PEP 695 type alias should be tracked");

    let ref_names: HashSet<String> = visitor.references.iter().map(|(n, _)| n.clone()).collect();
    assert!(
        ref_names.contains("UserId") || ref_names.contains("test.UserId"),
        "Alias use should be referenced"
    );
}

#[test]
fn test_halstead_keys_fn() {
    let code = r#"
dictionary = {"a": 1, "b": 2, "c": 3}
keys = dictionary.keys()
values = dictionary.values()
"#;
    visit_code!(code, visitor);

    // Verify "keys" definition exists
    let keys_def = visitor.definitions.iter().find(|d| d.simple_name == "keys");
    assert!(keys_def.is_some(), "keys definition should exist");

    // Verify references
    let ref_names: HashSet<String> = visitor.references.iter().map(|(n, _)| n.clone()).collect();

    // Debug print
    println!("References: {ref_names:?}");

    // "keys" (simple name) should NOT be referenced
    assert!(
        !ref_names.contains("keys"),
        "Simple name 'keys' is referenced!"
    );

    // ".keys" (attribute) SHOULD be referenced
    assert!(ref_names.contains(".keys"), "Attribute '.keys' missing!");
}
