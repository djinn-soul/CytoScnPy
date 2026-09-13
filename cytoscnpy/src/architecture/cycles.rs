//! Tarjan's Strongly Connected Components (SCC) and elementary cycle extraction.

use super::types::{CycleInfo, ModuleEdge, ModuleNode};
use std::collections::{HashMap, HashSet, VecDeque};

/// Detects all circular dependency cycles within the directed module graph.
pub fn detect_cycles(nodes: &[ModuleNode], edges: &[ModuleEdge]) -> (usize, usize, Vec<CycleInfo>) {
    let n = nodes.len();
    if n == 0 {
        return (0, 0, Vec::new());
    }

    // Build adjacency list and edge lookup: (from_id, to_id) -> &ModuleEdge
    let mut adj = vec![Vec::new(); n];
    let mut edge_lookup: HashMap<(usize, usize), &ModuleEdge> = HashMap::new();

    for edge in edges {
        if edge.from_id < n && edge.to_id < n && edge.from_id != edge.to_id {
            adj[edge.from_id].push(edge.to_id);
            edge_lookup.insert((edge.from_id, edge.to_id), edge);
        }
    }

    // Run Tarjan's SCC algorithm
    let sccs = run_tarjan(n, &adj);
    let mut cycle_infos = Vec::new();
    let mut largest_size = 0;

    for scc in sccs {
        if scc.len() < 2 {
            continue;
        }

        largest_size = largest_size.max(scc.len());
        let cycle_path = reconstruct_cycle_path(&scc, &adj);
        if let Some(info) = build_cycle_info(&cycle_path, nodes, &edge_lookup) {
            cycle_infos.push(info);
        }
    }

    // Sort cycles by largest size first, then alphabetically
    cycle_infos.sort_by(|a, b| {
        b.modules
            .len()
            .cmp(&a.modules.len())
            .then_with(|| a.modules.cmp(&b.modules))
    });

    let count = cycle_infos.len();
    (count, largest_size, cycle_infos)
}

struct Tarjan<'a> {
    adj: &'a [Vec<usize>],
    index: Vec<Option<usize>>,
    lowlink: Vec<usize>,
    on_stack: Vec<bool>,
    stack: Vec<usize>,
    time: usize,
    sccs: Vec<Vec<usize>>,
}

fn run_tarjan(n: usize, adj: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut t = Tarjan {
        adj,
        index: vec![None; n],
        lowlink: vec![0; n],
        on_stack: vec![false; n],
        stack: Vec::new(),
        time: 0,
        sccs: Vec::new(),
    };

    for i in 0..n {
        if t.index[i].is_none() {
            strongconnect(i, &mut t);
        }
    }

    t.sccs
}

fn strongconnect(v: usize, t: &mut Tarjan) {
    t.index[v] = Some(t.time);
    t.lowlink[v] = t.time;
    t.time += 1;
    t.stack.push(v);
    t.on_stack[v] = true;

    for &w in &t.adj[v] {
        if t.index[w].is_none() {
            strongconnect(w, t);
            t.lowlink[v] = t.lowlink[v].min(t.lowlink[w]);
        } else if t.on_stack[w] {
            if let Some(idx) = t.index[w] {
                t.lowlink[v] = t.lowlink[v].min(idx);
            }
        }
    }

    if t.lowlink[v] == t.index[v].unwrap_or(0) {
        let mut scc = Vec::new();
        while let Some(w) = t.stack.pop() {
            t.on_stack[w] = false;
            scc.push(w);
            if w == v {
                break;
            }
        }
        t.sccs.push(scc);
    }
}

/// Finds a directed cycle path within an SCC starting and ending at the canonical node.
fn reconstruct_cycle_path(scc: &[usize], adj: &[Vec<usize>]) -> Vec<usize> {
    let scc_set: HashSet<usize> = scc.iter().copied().collect();
    let &start = scc.iter().min().unwrap_or(&0);

    // BFS to find shortest path from any neighbor of `start` back to `start`
    for &first_hop in &adj[start] {
        if !scc_set.contains(&first_hop) {
            continue;
        }
        if first_hop == start {
            return vec![start, start];
        }

        // BFS from first_hop back to start
        let mut queue = VecDeque::new();
        let mut parent: HashMap<usize, usize> = HashMap::new();
        let mut visited: HashSet<usize> = HashSet::new();

        queue.push_back(first_hop);
        visited.insert(first_hop);

        let mut found = false;
        while let Some(curr) = queue.pop_front() {
            if curr == start {
                found = true;
                break;
            }
            for &nxt in &adj[curr] {
                if scc_set.contains(&nxt) && !visited.contains(&nxt) {
                    visited.insert(nxt);
                    parent.insert(nxt, curr);
                    queue.push_back(nxt);
                }
            }
        }

        if found {
            // Reconstruct path
            let mut path = vec![start];
            let mut curr = start;
            let mut segment = Vec::new();
            while curr != first_hop {
                if let Some(&p) = parent.get(&curr) {
                    segment.push(curr);
                    curr = p;
                } else {
                    break;
                }
            }
            segment.push(first_hop);
            segment.reverse();
            path.extend(segment);
            return path;
        }
    }

    // Fallback: return raw SCC elements closed with start
    let mut path = scc.to_vec();
    path.push(path[0]);
    path
}

fn build_cycle_info(
    path: &[usize],
    nodes: &[ModuleNode],
    edge_lookup: &HashMap<(usize, usize), &ModuleEdge>,
) -> Option<CycleInfo> {
    if path.len() < 2 {
        return None;
    }

    let mut module_names = Vec::new();
    let mut file_paths = Vec::new();
    let mut steps = Vec::new();
    let mut has_runtime_impact = false;

    for i in 0..path.len() - 1 {
        let from_id = path[i];
        let to_id = path[i + 1];
        let from_node = &nodes[from_id];
        let to_node = &nodes[to_id];

        module_names.push(from_node.name.clone());
        file_paths.push(from_node.file_path.clone());

        if let Some(edge) = edge_lookup.get(&(from_id, to_id)) {
            let primary_occ = edge.occurrences.first();
            let line = primary_occ.map_or(0, |o| o.line);
            let is_tc = primary_occ.is_some_and(|o| o.is_type_checking);
            let is_top = primary_occ.map_or(true, |o| o.is_top_level);

            if !is_tc && is_top {
                has_runtime_impact = true;
            }

            let tc_tag = if is_tc { " [TYPE_CHECKING]" } else { "" };
            let scope_tag = if is_top { "" } else { " (lazy/nested)" };
            steps.push(format!(
                "{} (line {}) imports {}{}{}",
                from_node.name, line, to_node.name, tc_tag, scope_tag
            ));
        } else {
            steps.push(format!("{} -> {}", from_node.name, to_node.name));
        }
    }

    // Close the cycle in the module name list
    if let Some(first) = module_names.first().cloned() {
        module_names.push(first);
    }
    if let Some(first_path) = file_paths.first().cloned() {
        file_paths.push(first_path);
    }

    Some(CycleInfo {
        modules: module_names,
        file_paths,
        has_runtime_impact,
        steps,
    })
}
