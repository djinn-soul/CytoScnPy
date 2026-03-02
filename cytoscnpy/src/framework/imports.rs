use rustc_hash::FxHashSet;
use std::sync::OnceLock;

/// Returns the set of framework import names used for detection.
pub fn get_framework_imports() -> &'static FxHashSet<&'static str> {
    static IMPORTS: OnceLock<FxHashSet<&'static str>> = OnceLock::new();
    IMPORTS.get_or_init(|| {
        let mut imports = FxHashSet::default();
        imports.insert("flask");
        imports.insert("fastapi");
        imports.insert("django");
        imports.insert("rest_framework");
        imports.insert("pydantic");
        imports.insert("celery");
        imports.insert("starlette");
        imports.insert("uvicorn");
        imports.insert("azure.functions");
        imports.insert("azure_functions");
        imports
    })
}
