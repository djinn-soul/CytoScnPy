use super::Cfg;
use rustc_hash::{FxHashMap, FxHashSet};

/// Result of Reaching Definitions analysis
#[derive(Debug, Default)]
pub struct FlowResult {
    /// For each block, the sets of definitions reaching its IN and OUT points
    pub block_results: FxHashMap<usize, BlockFlow>,
}

#[derive(Debug, Clone, Default)]
/// Flow information for a single basic block.
pub struct BlockFlow {
    /// Set of definitions that reach the entry of this block.
    pub in_set: FxHashSet<(String, usize)>,
    /// Set of definitions that reach the exit of this block.
    pub out_set: FxHashSet<(String, usize)>,
}

/// Reaching Definitions algorithm
///
/// # Panics
///
/// Panics if a block ID in the CFG is not found in the initialized results (should not happen).
#[must_use]
pub fn analyze_reaching_definitions(cfg: &Cfg) -> FlowResult {
    let mut results = FlowResult::default();

    // Initialize results for all blocks
    for block in &cfg.blocks {
        results.block_results.insert(block.id, BlockFlow::default());
    }

    // Worklist for fixed-point iteration
    let worklist: Vec<usize> = (0..cfg.blocks.len()).collect();
    let mut changed = true;

    while changed {
        changed = false;

        for &block_id in &worklist {
            let block = &cfg.blocks[block_id];

            // IN[B] = Union of OUT[P] for all predecessors P
            let mut new_in = FxHashSet::default();
            for &pred_id in &block.predecessors {
                if let Some(pred_flow) = results.block_results.get(&pred_id) {
                    for def in &pred_flow.out_set {
                        new_in.insert(def.clone());
                    }
                }
            }

            // OUT[B] = GEN[B] U (IN[B] - KILL[B])
            // GEN[B] = block.defs
            // KILL[B] = { (v, l) | v is defined in block and l' != l }

            let mut new_out = block.defs.clone();

            // Collect names defined in this block to "kill" incoming defs of same name
            let local_defs: FxHashSet<String> = block.defs.iter().map(|(n, _)| n.clone()).collect();

            for def in &new_in {
                if !local_defs.contains(&def.0) {
                    new_out.insert(def.clone());
                }
            }

            if let Some(flow) = results.block_results.get_mut(&block_id) {
                if flow.in_set != new_in || flow.out_set != new_out {
                    flow.in_set = new_in;
                    flow.out_set = new_out;
                    changed = true;
                }
            }
        }
    }

    results
}

impl FlowResult {
    /// Returns true if a definition at (name, line) reaches any usage.
    /// This is a simplified check: does the definition reach any block that uses this name?
    #[must_use]
    pub fn is_def_used(&self, cfg: &Cfg, name: &str, line: usize) -> bool {
        let def_tuple = (name.to_owned(), line);

        for (block_id, flow) in &self.block_results {
            let block = &cfg.blocks[*block_id];

            // Case 1: Use within the same block after the definition
            if block.defs.contains(&def_tuple)
                && block
                    .uses
                    .iter()
                    .any(|(u_name, u_line)| u_name == name && *u_line > line)
            {
                return true;
            }

            // Case 2: Use in a subsequent block reachable by this definition
            if flow.in_set.contains(&def_tuple)
                && block.uses.iter().any(|(u_name, _)| u_name == name)
            {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::Cfg;

    #[test]
    fn test_reaching_definitions_match() {
        let source = r#"
def test_func(command):
    match command:
        case ["load", filename]:
            print(filename)
        case ["save", filename]:
            pass
"#;
        let cfg = Cfg::from_source(source, "test_func").expect("Should parse");
        let results = analyze_reaching_definitions(&cfg);

        // Collect the "filename" definitions
        let defs: Vec<_> = cfg
            .blocks
            .iter()
            .flat_map(|b| b.defs.iter().filter(|(n, _)| n == "filename"))
            .cloned()
            .collect();
        assert_eq!(defs.len(), 2);

        let d1 = &defs[0];
        let d2 = &defs[1];

        // One should reach a use, the other should not.
        let u1 = results.is_def_used(&cfg, &d1.0, d1.1);
        let u2 = results.is_def_used(&cfg, &d2.0, d2.1);

        assert!(
            u1 != u2,
            "One definition should be used, the other unused. Got: d1_used={u1}, d2_used={u2}"
        );
    }

    #[test]
    fn test_dead_def_in_match_arm() {
        let source = r"
def f(x):
    match x:
        case [a, b]:
            print(a)
        case [a, c]:
            print(c)
";
        let cfg = Cfg::from_source(source, "f").unwrap();
        let results = analyze_reaching_definitions(&cfg);

        // a at line 4 (first case) should be used (print(a))
        // a at line 6 (second case) should be DEAD
        let used_a1 = results.is_def_used(&cfg, "a", 4);
        let used_a2 = results.is_def_used(&cfg, "a", 6);

        assert!(used_a1, "a at line 4 should be used");
        assert!(!used_a2, "a at line 6 should be dead");
    }
}
