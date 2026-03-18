//! Re-export chain regression tests.
#![allow(clippy::unwrap_used)]

use cytoscnpy::analyzer::CytoScnPy;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

fn project_tempdir() -> TempDir {
    let mut target_dir = std::env::current_dir().unwrap();
    target_dir.push("target");
    target_dir.push("test-reexport-chain-tmp");
    std::fs::create_dir_all(&target_dir).unwrap();
    tempfile::Builder::new()
        .prefix("reexport_chain_test_")
        .tempdir_in(target_dir)
        .unwrap()
}

#[test]
fn test_nested_reexport_chain_marks_imports_used_when_chain_is_complete() {
    let dir = project_tempdir();

    let app_pkg_core = dir.path().join("app").join("pkg").join("core");
    std::fs::create_dir_all(&app_pkg_core).unwrap();

    let mut app_init = File::create(dir.path().join("app").join("__init__.py")).unwrap();
    writeln!(app_init, "from .pkg import exposed_api").unwrap();

    let mut pkg_init =
        File::create(dir.path().join("app").join("pkg").join("__init__.py")).unwrap();
    writeln!(pkg_init, "from .core import exposed_api").unwrap();

    let mut core_init = File::create(
        dir.path()
            .join("app")
            .join("pkg")
            .join("core")
            .join("__init__.py"),
    )
    .unwrap();
    writeln!(core_init, "from .service import exposed_api").unwrap();

    let mut service = File::create(app_pkg_core.join("service.py")).unwrap();
    write!(service, "def exposed_api():\n    return 1\n").unwrap();

    let mut main = File::create(dir.path().join("main.py")).unwrap();
    write!(main, "from app import exposed_api\n_ = exposed_api()\n").unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = cytoscnpy.analyze(dir.path());

    assert!(
        result.unused_imports.is_empty(),
        "complete nested re-export chain should not report unused imports"
    );
    assert!(
        result.unused_functions.is_empty(),
        "re-exported callable should be counted as used"
    );
}

#[test]
fn test_nested_reexport_chain_reports_unused_imports_when_chain_is_broken() {
    let dir = project_tempdir();

    let app_pkg_core = dir.path().join("app").join("pkg").join("core");
    std::fs::create_dir_all(&app_pkg_core).unwrap();

    File::create(dir.path().join("app").join("__init__.py")).unwrap();

    let mut pkg_init =
        File::create(dir.path().join("app").join("pkg").join("__init__.py")).unwrap();
    writeln!(pkg_init, "from .core import exposed_api").unwrap();

    let mut core_init = File::create(
        dir.path()
            .join("app")
            .join("pkg")
            .join("core")
            .join("__init__.py"),
    )
    .unwrap();
    writeln!(core_init, "from .service import exposed_api").unwrap();

    let mut service = File::create(app_pkg_core.join("service.py")).unwrap();
    write!(service, "def exposed_api():\n    return 1\n").unwrap();

    let mut main = File::create(dir.path().join("main.py")).unwrap();
    write!(main, "from app import exposed_api\n_ = exposed_api()\n").unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = cytoscnpy.analyze(dir.path());

    let unused_import_names: Vec<&str> = result
        .unused_imports
        .iter()
        .map(|def| def.full_name.as_str())
        .collect();

    assert!(
        unused_import_names.contains(&"app.pkg.exposed_api"),
        "broken chain should report app.pkg.exposed_api as unused"
    );
}

#[test]
fn test_all_reexport_chain_marks_imports_and_source_used_without_external_callsite() {
    let dir = project_tempdir();

    let app_pkg_core = dir.path().join("app").join("pkg").join("core");
    std::fs::create_dir_all(&app_pkg_core).unwrap();

    let mut app_init = File::create(dir.path().join("app").join("__init__.py")).unwrap();
    writeln!(app_init, "from .pkg import exposed_api").unwrap();
    writeln!(app_init, "__all__ = [\"exposed_api\"]").unwrap();

    let mut pkg_init =
        File::create(dir.path().join("app").join("pkg").join("__init__.py")).unwrap();
    writeln!(pkg_init, "from .core import exposed_api").unwrap();
    writeln!(pkg_init, "__all__ = [\"exposed_api\"]").unwrap();

    let mut core_init = File::create(
        dir.path()
            .join("app")
            .join("pkg")
            .join("core")
            .join("__init__.py"),
    )
    .unwrap();
    writeln!(core_init, "from .service import exposed_api").unwrap();
    writeln!(core_init, "__all__ = [\"exposed_api\"]").unwrap();

    let mut service = File::create(app_pkg_core.join("service.py")).unwrap();
    write!(service, "def exposed_api():\n    return 1\n").unwrap();

    // Intentionally no call-site usage. The __all__ chain itself should preserve
    // the re-exported symbol and its import chain as used API surface.
    let mut main = File::create(dir.path().join("main.py")).unwrap();
    writeln!(main, "import app").unwrap();

    let mut cytoscnpy = CytoScnPy::default().with_confidence(60).with_tests(false);
    let result = cytoscnpy.analyze(dir.path());

    let unused_import_names: Vec<&str> = result
        .unused_imports
        .iter()
        .map(|def| def.full_name.as_str())
        .collect();
    let unused_function_names: Vec<&str> = result
        .unused_functions
        .iter()
        .map(|def| def.full_name.as_str())
        .collect();

    assert!(
        !unused_import_names.contains(&"app.exposed_api")
            && !unused_import_names.contains(&"app.pkg.exposed_api")
            && !unused_import_names.contains(&"app.pkg.core.exposed_api"),
        "__all__ re-export chain should keep import bindings marked as used"
    );
    assert!(
        !unused_function_names.contains(&"app.pkg.core.service.exposed_api"),
        "__all__ re-export chain should keep source callable marked as used"
    );
}
