use crate::taint::call_graph::CallGraph;
use crate::visitor::Definition;
use rustc_hash::{FxHashMap, FxHashSet};

pub(super) struct ReachabilityContext {
    pub(super) implicitly_used_methods: FxHashSet<String>,
    pub(super) reachable_nodes: FxHashSet<String>,
}

pub(super) fn build_reachability(
    definitions: &[Definition],
    protocol_methods: &FxHashMap<String, FxHashSet<String>>,
    dynamic_imported_modules: &FxHashSet<String>,
    call_graph: &CallGraph,
) -> ReachabilityContext {
    let class_methods = collect_class_methods(definitions);
    let implicitly_used_methods = collect_implicitly_used_methods(&class_methods, protocol_methods);
    let reachable_nodes = collect_reachable_nodes(
        definitions,
        &class_methods,
        &implicitly_used_methods,
        dynamic_imported_modules,
        call_graph,
    );

    ReachabilityContext {
        implicitly_used_methods,
        reachable_nodes,
    }
}

fn collect_class_methods(definitions: &[Definition]) -> FxHashMap<String, FxHashSet<String>> {
    let mut class_methods: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();

    for def in definitions {
        if def.def_type != "method" {
            continue;
        }

        if let Some(parent) = def.full_name.rfind('.').map(|idx| &def.full_name[..idx]) {
            class_methods
                .entry(parent.to_owned())
                .or_default()
                .insert(def.simple_name.clone());
        }
    }

    class_methods
}

fn collect_implicitly_used_methods(
    class_methods: &FxHashMap<String, FxHashSet<String>>,
    protocol_methods: &FxHashMap<String, FxHashSet<String>>,
) -> FxHashSet<String> {
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

    for (class_name, methods) in class_methods {
        let mut candidate_protocols: FxHashSet<&String> = FxHashSet::default();
        for method in methods {
            if let Some(protocols) = method_to_protocols.get(method) {
                for protocol in protocols {
                    candidate_protocols.insert(protocol);
                }
            }
        }

        for proto_name in candidate_protocols {
            if let Some(proto_methods) = protocol_methods.get(proto_name) {
                let intersection_count = methods.intersection(proto_methods).count();
                let proto_len = proto_methods.len();
                if proto_len > 0 && intersection_count >= 3 {
                    let ratio = intersection_count as f64 / proto_len as f64;
                    if ratio >= 0.7 {
                        for method in methods.intersection(proto_methods) {
                            implicitly_used_methods.insert(format!("{class_name}.{method}"));
                        }
                    }
                }
            }
        }
    }

    implicitly_used_methods
}

fn collect_reachable_nodes(
    definitions: &[Definition],
    class_methods: &FxHashMap<String, FxHashSet<String>>,
    implicitly_used_methods: &FxHashSet<String>,
    dynamic_imported_modules: &FxHashSet<String>,
    call_graph: &CallGraph,
) -> FxHashSet<String> {
    let mut roots = FxHashSet::default();
    let mut method_simple_to_full: FxHashMap<String, Vec<String>> = FxHashMap::default();

    for def in definitions {
        if def.def_type == "method" {
            method_simple_to_full
                .entry(def.simple_name.clone())
                .or_default()
                .push(def.full_name.clone());
        }

        if def.is_exported
            || def.is_framework_managed
            || def.confidence == 0
            || implicitly_used_methods.contains(&def.full_name)
            || dynamic_imported_modules.iter().any(|module| {
                !module.is_empty()
                    && (def.full_name == *module
                        || def.full_name.starts_with(&format!("{module}.")))
            })
        {
            roots.insert(def.full_name.clone());

            if def.def_type == "class" {
                if let Some(methods) = class_methods.get(&def.full_name) {
                    for method in methods {
                        if !method.starts_with('_') {
                            roots.insert(format!("{}.{}", def.full_name, method));
                        }
                    }
                }
            }
        }
    }

    for (name, node) in &call_graph.nodes {
        if node.is_root {
            roots.insert(name.clone());
        }
    }

    let mut reachable_nodes = FxHashSet::default();
    let mut stack: Vec<String> = roots.into_iter().collect();

    while let Some(current) = stack.pop() {
        if !reachable_nodes.insert(current.clone()) {
            continue;
        }

        if let Some(node) = call_graph.nodes.get(&current) {
            for call in &node.calls {
                if let Some(attr_name) = call.strip_prefix('.') {
                    if let Some(methods) = method_simple_to_full.get(attr_name) {
                        for method in methods {
                            if !reachable_nodes.contains(method) {
                                stack.push(method.clone());
                            }
                        }
                    }
                } else if call_graph.nodes.contains_key(call) && !reachable_nodes.contains(call) {
                    stack.push(call.clone());
                }
            }
        }
    }

    reachable_nodes
}
