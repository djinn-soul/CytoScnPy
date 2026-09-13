//! Layering analysis: God modules and bidirectional package group dependencies.

use super::types::{BidirectionalGroupDep, GodModuleInfo, ModuleEdge, ModuleNode};
use std::collections::{HashMap, HashSet};

/// Identifies god modules (excessive fan-in or coupling hubs) in the project.
pub fn find_god_modules<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    nodes: &[ModuleNode],
    fan_in_map: &HashMap<String, usize, S1>,
    fan_out_map: &HashMap<String, usize, S2>,
) -> Vec<GodModuleInfo> {
    let total = nodes.len();
    if total < 3 {
        return Vec::new();
    }

    let mut god_modules = Vec::new();

    for node in nodes {
        let fan_in = fan_in_map.get(&node.name).copied().unwrap_or(0);
        let fan_out = fan_out_map.get(&node.name).copied().unwrap_or(0);
        let percentage = (fan_in as f64 / (total - 1) as f64) * 100.0;

        // Condition 1: Imported by >= 40% of all other modules (when total >= 5)
        if total >= 5 && percentage >= 40.0 && fan_in >= 3 {
            god_modules.push(GodModuleInfo {
                module_name: node.name.clone(),
                file_path: node.file_path.clone(),
                fan_in,
                fan_in_percentage: percentage,
                fan_out,
                reason: format!("Imported by {fan_in} modules ({percentage:.0}% of codebase)"),
            });
            continue;
        }

        // Condition 2: Absolute high fan-in in larger repositories
        if total >= 15 && fan_in >= 10 {
            god_modules.push(GodModuleInfo {
                module_name: node.name.clone(),
                file_path: node.file_path.clone(),
                fan_in,
                fan_in_percentage: percentage,
                fan_out,
                reason: format!("High coupling concentration: imported by {fan_in} modules"),
            });
            continue;
        }

        // Condition 3: Central bottleneck hub (both high incoming and outgoing coupling)
        if fan_in >= 5 && fan_out >= 8 {
            god_modules.push(GodModuleInfo {
                module_name: node.name.clone(),
                file_path: node.file_path.clone(),
                fan_in,
                fan_in_percentage: percentage,
                fan_out,
                reason: format!(
                    "Coupling hub: fan-in={fan_in}, fan-out={fan_out} (violates single responsibility)"
                ),
            });
        }
    }

    god_modules.sort_by_key(|a| std::cmp::Reverse(a.fan_in));
    god_modules
}

/// Detects bidirectional dependencies between distinct top-level package groups.
pub fn find_bidirectional_group_deps(
    nodes: &[ModuleNode],
    edges: &[ModuleEdge],
) -> Vec<BidirectionalGroupDep> {
    // Map node_id -> package_group
    let node_groups: HashMap<usize, &str> = nodes
        .iter()
        .map(|n| (n.id, n.package_group.as_str()))
        .collect();

    // Map (group_a, group_b) -> (count, sample_module_edge)
    let mut group_edges: HashMap<(String, String), (usize, String)> = HashMap::new();

    for edge in edges {
        let Some(&from_group) = node_groups.get(&edge.from_id) else {
            continue;
        };
        let Some(&to_group) = node_groups.get(&edge.to_id) else {
            continue;
        };

        if from_group != to_group && from_group != "<root>" && to_group != "<root>" {
            let key = (from_group.to_owned(), to_group.to_owned());
            let entry = group_edges.entry(key).or_insert((0, String::new()));
            entry.0 += 1;
            if entry.1.is_empty() {
                let line = edge.occurrences.first().map_or(0, |o| o.line);
                entry.1 = format!("{}:{} -> {}", edge.from_name, line, edge.to_name);
            }
        }
    }

    let mut checked_pairs: HashSet<(String, String)> = HashSet::new();
    let mut violations = Vec::new();

    for ((group_a, group_b), (count_a_to_b, sample_a)) in &group_edges {
        let reverse_key = (group_b.clone(), group_a.clone());
        let canonical_pair = if group_a < group_b {
            (group_a.clone(), group_b.clone())
        } else {
            (group_b.clone(), group_a.clone())
        };

        if checked_pairs.insert(canonical_pair) {
            if let Some(&(count_b_to_a, ref sample_b)) = group_edges.get(&reverse_key) {
                violations.push(BidirectionalGroupDep {
                    group_a: group_a.clone(),
                    group_b: group_b.clone(),
                    edges_a_to_b: *count_a_to_b,
                    edges_b_to_a: count_b_to_a,
                    sample_a_to_b: sample_a.clone(),
                    sample_b_to_a: sample_b.clone(),
                });
            }
        }
    }

    violations.sort_by_key(|a| std::cmp::Reverse(a.edges_a_to_b + a.edges_b_to_a));
    violations
}
