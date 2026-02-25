use std::collections::HashMap;
use std::sync::OnceLock;

/// Returns confidence penalties applied by heuristic keys.
pub fn get_penalties() -> &'static HashMap<&'static str, u8> {
    static PENALTIES: OnceLock<HashMap<&'static str, u8>> = OnceLock::new();
    PENALTIES.get_or_init(|| {
        let mut penalties = HashMap::new();
        penalties.insert("private_name", 80);
        penalties.insert("dunder_or_magic", 100);
        penalties.insert("underscored_var", 100);
        penalties.insert("in_init_file", 15);
        penalties.insert("dynamic_module", 40);
        penalties.insert("test_related", 100);
        penalties.insert("framework_magic", 40);
        penalties.insert("framework_managed", 50);
        penalties.insert("mixin_class", 60);
        penalties.insert("base_abstract_interface", 50);
        penalties.insert("adapter_class", 30);
        penalties.insert("lifecycle_hook", 30);
        penalties.insert("compose_method", 40);
        penalties.insert("type_checking_import", 100);
        penalties.insert("module_constant", 5);
        penalties
    })
}
