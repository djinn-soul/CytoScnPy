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
