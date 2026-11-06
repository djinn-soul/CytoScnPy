use super::protocols::{
    collect_class_methods, collect_implicitly_used_methods, collect_runtime_protocol_hints,
};
use crate::taint::call_graph::CallGraph;
use crate::visitor::Definition;
use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};

/// Reachability evidence for each definition, indexed like the definitions
/// passed to [`build_reachability`].
pub(super) struct ReachabilityContext {
    pub(super) flags: Vec<ReachabilityFlags>,
}

#[derive(Clone, Copy)]
pub(super) struct ReachabilityFlags {
    /// The method implements a protocol the class structurally satisfies.
    pub(super) implicitly_used: bool,
    /// The definition is reachable from a root through the call graph.
    pub(super) reachable: bool,
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
    let reachable = collect_reachable_nodes(
        definitions,
        &class_methods,
        &implicitly_used_methods,
        dynamic_imported_modules,
        call_graph,
        include_tests,
    );

    let flags = definitions
        .par_iter()
        .zip(reachable)
        .map(|(def, reachable)| ReachabilityFlags {
            implicitly_used: implicitly_used_methods.contains(&def.full_name),
            reachable,
        })
        .collect();
    ReachabilityContext { flags }
}

fn collect_reachable_nodes(
    definitions: &[Definition],
    class_methods: &FxHashMap<String, FxHashSet<String>>,
    implicitly_used_methods: &FxHashSet<String>,
    dynamic_imported_modules: &FxHashSet<String>,
    call_graph: &CallGraph,
    include_tests: bool,
) -> Vec<bool> {
    // Names borrow from `definitions`, `call_graph` and `root_methods`, so
    // the walk allocates nothing per node.
    let mut method_simple_to_full: FxHashMap<&str, Vec<&str>> = FxHashMap::default();
    let mut callable_simple_to_full: FxHashMap<&str, Vec<&str>> = FxHashMap::default();
    let mut class_bases: FxHashMap<&str, &[String]> = FxHashMap::default();
    let mut class_simple_to_full: FxHashMap<&str, Vec<&str>> = FxHashMap::default();
    let mut roots: Vec<&str> = Vec::new();
    let mut root_classes: Vec<&str> = Vec::new();

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
                root_classes.push(&def.full_name);
            }
        }
    }

    // A class extending a base from outside the project has its methods,
    // private hooks included, called by that base (`textwrap.TextWrapper`
    // calls `_handle_long_word`). A base matching only the class itself, as
    // in `class TextWrapper(textwrap.TextWrapper)`, is external too.
    let extends_external = |class: &str| {
        class_bases.get(class).is_some_and(|bases| {
            bases.iter().any(|base| {
                !matches!(base.as_str(), "object" | "ABC" | "Generic")
                    && class_simple_to_full
                        .get(base.as_str())
                        .map_or(true, |candidates| candidates.iter().all(|c| *c == class))
            })
        })
    };
    let class_method_names = |class: &str, include_private: bool| -> Vec<String> {
        class_methods.get(class).map_or_else(Vec::new, |methods| {
            methods
                .iter()
                .filter(|method| include_private || !method.starts_with('_'))
                .map(|method| format!("{class}.{method}"))
                .collect()
        })
    };
    let mut root_methods: Vec<String> = Vec::new();
    for class in &root_classes {
        root_methods.extend(class_method_names(class, extends_external(class)));
    }
    let external_class_methods: FxHashMap<&str, Vec<String>> = class_bases
        .keys()
        .filter(|class| extends_external(class))
        .map(|class| (*class, class_method_names(class, true)))
        .collect();

    roots.extend(
        call_graph
            .nodes
            .iter()
            .filter(|(_, node)| node.is_root)
            .map(|(name, _)| name.as_str()),
    );
    roots.extend(root_methods.iter().map(String::as_str));

    let mut reachable_nodes: FxHashSet<&str> =
        FxHashSet::with_capacity_and_hasher(roots.len(), rustc_hash::FxBuildHasher);
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

        if let Some(methods) = external_class_methods.get(current) {
            stack.extend(
                methods
                    .iter()
                    .map(String::as_str)
                    .filter(|method| !reachable_nodes.contains(method)),
            );
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
            // References (callbacks, registries, nested functions) keep their
            // targets alive just as calls do.
            for call in node.calls.iter().chain(&node.references) {
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
                } else if let Some((_, simple)) = call.rsplit_once('.') {
                    // `a.helper()` after `from . import a`: match definitions
                    // whose qualified name ends with `.a.helper`.
                    if !expanded_callables.insert(call) {
                        continue;
                    }
                    if let Some(candidates) = callable_simple_to_full.get(simple) {
                        stack.extend(candidates.iter().filter(|candidate| {
                            candidate
                                .strip_suffix(call.as_str())
                                .is_some_and(|prefix| prefix.ends_with('.'))
                                && !reachable_nodes.contains(*candidate)
                        }));
                    }
                }
            }
        }
    }

    definitions
        .par_iter()
        .map(|def| reachable_nodes.contains(def.full_name.as_str()))
        .collect()
}
