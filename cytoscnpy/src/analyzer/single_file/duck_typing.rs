use crate::visitor::Definition;
use rustc_hash::{FxHashMap, FxHashSet};

pub(super) fn apply_duck_typing_usage(
    definitions: &mut [Definition],
    protocol_methods: &FxHashMap<String, FxHashSet<String>>,
) {
    let mut class_methods: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();

    for def in definitions.iter() {
        if def.def_type == "method" {
            if let Some(parent) = def.full_name.rfind('.').map(|idx| &def.full_name[..idx]) {
                class_methods
                    .entry(parent.to_owned())
                    .or_default()
                    .insert(def.simple_name.clone());
            }
        }
    }

    let mut method_to_protocols: FxHashMap<String, Vec<&String>> = FxHashMap::default();
    for (proto_name, methods) in protocol_methods {
        for method in methods {
            method_to_protocols
                .entry(method.clone())
                .or_default()
                .push(proto_name);
        }
    }

    let mut implicitly_used_methods: FxHashSet<String> = FxHashSet::default();
    for (class_name, methods) in &class_methods {
        let mut candidate_protocols: FxHashSet<&String> = FxHashSet::default();
        for method in methods {
            if let Some(protocols) = method_to_protocols.get(method) {
                for protocol in protocols {
                    candidate_protocols.insert(protocol);
                }
            }
        }

        for proto_name in candidate_protocols {
            if *class_name == **proto_name {
                continue;
            }
            if let Some(proto_defs) = protocol_methods.get(proto_name) {
                let intersection_count = methods.intersection(proto_defs).count();
                let proto_len = proto_defs.len();
                if proto_len > 0 && intersection_count >= 3 {
                    let ratio = intersection_count as f64 / proto_len as f64;
                    if ratio >= 0.7 {
                        for method in methods.intersection(proto_defs) {
                            implicitly_used_methods.insert(format!("{class_name}.{method}"));
                        }
                    }
                }
            }
        }
    }

    for def in definitions.iter_mut() {
        if implicitly_used_methods.contains(&def.full_name) {
            def.references = std::cmp::max(def.references, 1);
        }
    }
}
