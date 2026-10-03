//! Graph construction, dependency aggregation, and metrics computation.

use super::collector::collect_raw_imports;
use super::cycles::detect_cycles;
use super::layering::{find_bidirectional_group_deps, find_god_modules};
use super::resolver::{ModuleResolver, ResolvedTarget};
use super::types::{ArchitectureGraphResult, ArchitectureGraphStats, ImportOccurrence, ModuleEdge};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;

/// Builds the complete module architecture graph from Python files.
pub fn build_architecture_graph(files: &[PathBuf], roots: &[PathBuf]) -> ArchitectureGraphResult {
    let resolver = ModuleResolver::build(files, roots);
    let total_modules = resolver.nodes.len();

    if total_modules == 0 {
        return ArchitectureGraphResult {
            stats: ArchitectureGraphStats::default(),
            nodes: Vec::new(),
            edges: Vec::new(),
            external_imports: HashMap::new(),
            fan_in_map: HashMap::new(),
            fan_out_map: HashMap::new(),
        };
    }

    // Step 1: Parallel extraction of raw imports from AST across all files
    let raw_imports_per_file: Vec<Vec<super::collector::RawImport>> = resolver
        .nodes
        .par_iter()
        .map(|node| collect_raw_imports(&node.file_path))
        .collect();

    // Step 2: Resolve edges into internal project dependencies vs external packages
    let mut edge_map: HashMap<(usize, usize), Vec<ImportOccurrence>> = HashMap::new();
    let mut external_imports: HashMap<String, usize> = HashMap::new();

    for (node_id, raw_imports) in raw_imports_per_file.into_iter().enumerate() {
        for raw in raw_imports {
            let target = resolver.resolve_import(
                node_id,
                raw.module.as_deref(),
                raw.imported_symbol.as_deref(),
                raw.level,
            );

            match target {
                Some(ResolvedTarget::Internal(target_id)) => {
                    // Avoid self-dependency loops on the module level
                    if node_id != target_id {
                        let occ = ImportOccurrence {
                            line: raw.line,
                            column: raw.column,
                            is_type_checking: raw.is_type_checking,
                            is_top_level: raw.is_top_level,
                            imported_symbol: raw.imported_symbol,
                        };
                        edge_map.entry((node_id, target_id)).or_default().push(occ);
                    }
                }
                Some(ResolvedTarget::External(pkg)) => {
                    *external_imports.entry(pkg).or_insert(0) += 1;
                }
                None => {}
            }
        }
    }

    // Step 3: Construct unique ModuleEdge list
    let mut edges: Vec<ModuleEdge> = edge_map
        .into_iter()
        .map(|((from_id, to_id), occurrences)| ModuleEdge {
            from_id,
            to_id,
            from_name: resolver.nodes[from_id].name.clone(),
            to_name: resolver.nodes[to_id].name.clone(),
            occurrences,
        })
        .collect();

    // Sort edges consistently
    edges.sort_by(|a, b| {
        a.from_name
            .cmp(&b.from_name)
            .then_with(|| a.to_name.cmp(&b.to_name))
    });

    // Step 4: Compute Fan-in and Fan-out per module
    let mut fan_in_counts = vec![0usize; total_modules];
    let mut fan_out_counts = vec![0usize; total_modules];

    for edge in &edges {
        fan_out_counts[edge.from_id] += 1;
        fan_in_counts[edge.to_id] += 1;
    }

    let mut fan_in_map = HashMap::new();
    let mut fan_out_map = HashMap::new();
    for (i, node) in resolver.nodes.iter().enumerate() {
        fan_in_map.insert(node.name.clone(), fan_in_counts[i]);
        fan_out_map.insert(node.name.clone(), fan_out_counts[i]);
    }

    // Step 5: Compute Fan-in and Fan-out summary metrics
    let total_fan_in: usize = fan_in_counts.iter().sum();
    let total_fan_out: usize = fan_out_counts.iter().sum();
    let avg_fan_in = total_fan_in as f64 / total_modules as f64;
    let avg_fan_out = total_fan_out as f64 / total_modules as f64;

    let (max_fan_in, max_fan_in_node) = fan_in_counts
        .iter()
        .enumerate()
        .max_by_key(|(_, &cnt)| cnt)
        .map(|(idx, &cnt)| (cnt, Some(resolver.nodes[idx].name.clone())))
        .unwrap_or((0, None));

    let (max_fan_out, max_fan_out_node) = fan_out_counts
        .iter()
        .enumerate()
        .max_by_key(|(_, &cnt)| cnt)
        .map(|(idx, &cnt)| (cnt, Some(resolver.nodes[idx].name.clone())))
        .unwrap_or((0, None));

    // Step 6: Cycle detection (Tarjan's SCC)
    let (circular_count, largest_cycle_size, cycles) = detect_cycles(&resolver.nodes, &edges);

    // Step 7: God modules and bidirectional package group dependencies
    let god_modules = find_god_modules(&resolver.nodes, &fan_in_map, &fan_out_map);
    let bidirectional_group_deps = find_bidirectional_group_deps(&resolver.nodes, &edges);

    let total_external_imports = external_imports.values().sum();

    let stats = ArchitectureGraphStats {
        total_modules,
        total_internal_edges: edges.len(),
        total_external_imports,
        avg_fan_in,
        max_fan_in,
        max_fan_in_module: max_fan_in_node,
        avg_fan_out,
        max_fan_out,
        max_fan_out_module: max_fan_out_node,
        circular_dependency_count: circular_count,
        largest_cycle_size,
        cycles,
        god_modules,
        bidirectional_group_deps,
    };

    ArchitectureGraphResult {
        stats,
        nodes: resolver.nodes,
        edges,
        external_imports,
        fan_in_map,
        fan_out_map,
    }
}
