use crate::taint::call_graph::CallGraph;
use crate::visitor::Definition;
use rustc_hash::{FxHashMap, FxHashSet};

pub(super) const RUNTIME_PROTOCOL_REF_PREFIX: &str = "__csp_runtime_protocol__.";

pub(super) struct ReachabilityContext {
    pub(super) implicitly_used_methods: FxHashSet<String>,
    pub(super) reachable_nodes: FxHashSet<String>,
    pub(super) explicitly_called_nodes: FxHashSet<String>,
}

pub(super) fn build_reachability(
    definitions: &[Definition],
    ref_counts: &FxHashMap<String, usize>,
    protocol_methods: &FxHashMap<String, FxHashSet<String>>,
    dynamic_imported_modules: &FxHashSet<String>,
    call_graph: &CallGraph,
    include_tests: bool,
) -> ReachabilityContext {
    let class_methods = collect_class_methods(definitions);
    let runtime_protocol_hints = collect_runtime_protocol_hints(ref_counts);
    let implicitly_used_methods =
        collect_implicitly_used_methods(&class_methods, protocol_methods, &runtime_protocol_hints);
    let reachable_nodes = collect_reachable_nodes(
        definitions,
        &class_methods,
        &implicitly_used_methods,
        dynamic_imported_modules,
        call_graph,
        include_tests,
    );
    let explicitly_called_nodes = collect_explicitly_called_nodes(call_graph);

    ReachabilityContext {
        implicitly_used_methods,
        reachable_nodes,
        explicitly_called_nodes,
    }
}

fn collect_explicitly_called_nodes(call_graph: &CallGraph) -> FxHashSet<String> {
    let mut called = FxHashSet::default();
    for node in call_graph.nodes.values() {
        for call in &node.calls {
            if call.starts_with('.') {
                continue;
            }
            called.insert(call.clone());
        }
    }
    called
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
    runtime_protocol_hints: &FxHashSet<String>,
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

    // Whether a protocol is runtime-checked depends only on the protocol, so
    // scan the hints once per protocol rather than once per class.
    let runtime_checked_protocols: FxHashSet<&str> = protocol_methods
        .keys()
        .filter(|proto_name| protocol_hint_matches(proto_name, runtime_protocol_hints))
        .map(String::as_str)
        .collect();

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
                let runtime_checked = runtime_checked_protocols.contains(proto_name.as_str());
                let passes_similarity_threshold =
                    similarity_threshold_met(intersection_count, proto_len);
                let passes_runtime_threshold = runtime_checked && intersection_count > 0;

                if passes_similarity_threshold || passes_runtime_threshold {
                    for method in methods.intersection(proto_methods) {
                        implicitly_used_methods.insert(format!("{class_name}.{method}"));
                    }
                }
            }
        }
    }

    implicitly_used_methods
}

fn collect_runtime_protocol_hints(ref_counts: &FxHashMap<String, usize>) -> FxHashSet<String> {
    ref_counts
        .keys()
        .filter_map(|key| key.strip_prefix(RUNTIME_PROTOCOL_REF_PREFIX))
        .map(std::borrow::ToOwned::to_owned)
        .collect()
}

/// Returns `true` when a class's method overlap with a protocol is large enough
/// to treat the class as an implicit implementor of that protocol.
///
/// Thresholds: at least 3 shared methods **and** ≥ 70 % of the protocol surface.
fn similarity_threshold_met(intersection_count: usize, proto_len: usize) -> bool {
    const MIN_SHARED_METHODS: usize = 3;
    const MIN_COVERAGE_RATIO: f64 = 0.7;

    proto_len > 0
        && intersection_count >= MIN_SHARED_METHODS
        && (intersection_count as f64 / proto_len as f64) >= MIN_COVERAGE_RATIO
}

fn protocol_hint_matches(proto_name: &str, runtime_protocol_hints: &FxHashSet<String>) -> bool {
    if runtime_protocol_hints.contains(proto_name) {
        return true;
    }
    // Build the suffix once and reuse, rather than `format!` per hint.
    let suffix_len = proto_name.len() + 1;
    runtime_protocol_hints.iter().any(|hint| {
        hint.len() > suffix_len
            && hint.as_bytes()[hint.len() - suffix_len] == b'.'
            && hint.ends_with(proto_name)
    })
}

fn collect_reachable_nodes(
    definitions: &[Definition],
    class_methods: &FxHashMap<String, FxHashSet<String>>,
    implicitly_used_methods: &FxHashSet<String>,
    dynamic_imported_modules: &FxHashSet<String>,
    call_graph: &CallGraph,
    include_tests: bool,
) -> FxHashSet<String> {
    // Names borrow from `definitions`, `call_graph` and `root_methods`, so
    // the walk allocates nothing per node.
    let mut method_simple_to_full: FxHashMap<&str, Vec<&str>> = FxHashMap::default();
    let mut callable_simple_to_full: FxHashMap<&str, Vec<&str>> = FxHashMap::default();
    let mut class_bases: FxHashMap<&str, &[String]> = FxHashMap::default();
    let mut class_simple_to_full: FxHashMap<&str, Vec<&str>> = FxHashMap::default();
    let mut roots: Vec<&str> = Vec::new();
    let mut root_methods: Vec<String> = Vec::new();

    // Hoist prefix-form once instead of `format!("{module}.")` per definition.
    let dynamic_module_prefixes: Vec<(String, String)> = dynamic_imported_modules
        .iter()
        .filter(|module| !module.is_empty())
        .map(|module| (module.clone(), format!("{module}.")))
        .collect();

    for def in definitions {
        if def.def_type == "method" {
            method_simple_to_full
                .entry(&def.simple_name)
                .or_default()
                .push(&def.full_name);
        } else if def.def_type == "function" || def.def_type == "class" {
            callable_simple_to_full
                .entry(&def.simple_name)
                .or_default()
                .push(&def.full_name);
            if def.def_type == "class" {
                class_simple_to_full
                    .entry(&def.simple_name)
                    .or_default()
                    .push(&def.full_name);
                if !def.base_classes.is_empty() {
                    class_bases.insert(&def.full_name, &def.base_classes);
                }
            }
        }

        if def.is_exported
            || def.is_framework_managed
            || def.confidence == 0
            || implicitly_used_methods.contains(&def.full_name)
            || (include_tests && crate::utils::is_test_path(&def.file.to_string_lossy()))
            || dynamic_module_prefixes.iter().any(|(module, prefix)| {
                def.full_name == *module || def.full_name.starts_with(prefix)
            })
        {
            roots.push(&def.full_name);

            if def.def_type == "class" {
                if let Some(methods) = class_methods.get(&def.full_name) {
                    for method in methods {
                        if !method.starts_with('_') {
                            root_methods.push(format!("{}.{}", def.full_name, method));
                        }
                    }
                }
            }
        }
    }

    roots.extend(
        call_graph
            .nodes
            .iter()
            .filter(|(_, node)| node.is_root)
            .map(|(name, _)| name.as_str()),
    );
    roots.extend(root_methods.iter().map(String::as_str));

    let mut reachable_nodes: FxHashSet<&str> = FxHashSet::default();
    let mut stack = roots;
    // Expanding a simple name pushes every candidate at once, so later
    // expansions of the same name add nothing. Common names such as `.get`
    // would otherwise rescan thousands of candidates at every call site.
    let mut expanded_attrs: FxHashSet<&str> = FxHashSet::default();
    let mut expanded_callables: FxHashSet<&str> = FxHashSet::default();
    let mut expanded_classes: FxHashSet<&str> = FxHashSet::default();

    while let Some(current) = stack.pop() {
        if !reachable_nodes.insert(current) {
            continue;
        }

        // Propagate reachability through base classes: a reachable class implicitly
        // keeps its bases alive (subclassing is a usage edge the call graph misses).
        if let Some(bases) = class_bases.get(current) {
            for base in *bases {
                if call_graph.nodes.contains_key(base) && !reachable_nodes.contains(base.as_str()) {
                    stack.push(base);
                }
                if !expanded_classes.insert(base) {
                    continue;
                }
                if let Some(candidates) = class_simple_to_full.get(base.as_str()) {
                    stack.extend(
                        candidates
                            .iter()
                            .filter(|candidate| !reachable_nodes.contains(*candidate)),
                    );
                }
            }
        }

        if let Some(node) = call_graph.nodes.get(current) {
            for call in &node.calls {
                if let Some(attr_name) = call.strip_prefix('.') {
                    if !expanded_attrs.insert(attr_name) {
                        continue;
                    }
                    if let Some(methods) = method_simple_to_full.get(attr_name) {
                        stack.extend(
                            methods
                                .iter()
                                .filter(|method| !reachable_nodes.contains(*method)),
                        );
                    }
                } else if call_graph.nodes.contains_key(call) {
                    if !reachable_nodes.contains(call.as_str()) {
                        stack.push(call);
                    }
                } else if !call.contains('.') {
                    if !expanded_callables.insert(call) {
                        continue;
                    }
                    if let Some(candidates) = callable_simple_to_full.get(call.as_str()) {
                        stack.extend(
                            candidates
                                .iter()
                                .filter(|candidate| !reachable_nodes.contains(*candidate)),
                        );
                    }
                }
            }
        }
    }

    reachable_nodes.into_iter().map(str::to_owned).collect()
}
