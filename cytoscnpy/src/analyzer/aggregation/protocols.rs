//! Structural protocol matching: methods a class implements implicitly.

use crate::visitor::Definition;
use rustc_hash::{FxHashMap, FxHashSet};

const RUNTIME_PROTOCOL_REF_PREFIX: &str = "__csp_runtime_protocol__.";

pub(super) fn collect_class_methods(
    definitions: &[Definition],
) -> FxHashMap<String, FxHashSet<String>> {
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

pub(super) fn collect_implicitly_used_methods(
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

pub(super) fn collect_runtime_protocol_hints(
    ref_counts: &FxHashMap<String, usize>,
) -> FxHashSet<String> {
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
