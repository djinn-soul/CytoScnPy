use super::*;
use rustc_hash::FxHashSet;

#[test]
fn test_cfg_fingerprint() {
    let cfg = Cfg {
        blocks: vec![BasicBlock {
            id: 0,
            statements: vec![
                StmtRef {
                    line: 1,
                    kind: StmtKind::If,
                },
                StmtRef {
                    line: 2,
                    kind: StmtKind::For,
                },
            ],
            successors: vec![],
            predecessors: vec![],
            loop_depth: 1,
            defs: FxHashSet::default(),
            uses: FxHashSet::default(),
        }],
        entry: 0,
        exits: vec![0],
    };

    let fp = cfg.fingerprint();
    assert_eq!(fp.block_count, 1);
    assert_eq!(fp.max_loop_depth, 1);
    assert_eq!(fp.branch_count, 1);
    assert_eq!(fp.loop_count, 1);
}

#[test]
fn test_cfg_from_source_simple() {
    let source = "def simple_func():\n    x = 1\n    y = 2\n    return x + y\n";
    let cfg = Cfg::from_source(source, "simple_func").expect("Should parse");
    assert!(!cfg.blocks.is_empty());
    let fp = cfg.fingerprint();
    assert_eq!(fp.branch_count, 0);
    assert_eq!(fp.loop_count, 0);

    let entry_block = &cfg.blocks[0];
    assert!(entry_block.defs.iter().any(|(n, _)| n == "x"));
    assert!(entry_block.defs.iter().any(|(n, _)| n == "y"));
    assert!(entry_block.uses.iter().any(|(n, _)| n == "x"));
    assert!(entry_block.uses.iter().any(|(n, _)| n == "y"));
}

#[test]
fn test_cfg_from_source_with_if() {
    let source =
        "def func_with_if(x):\n    if x > 0:\n        return 1\n    else:\n        return -1\n";
    let cfg = Cfg::from_source(source, "func_with_if").expect("Should parse");
    let fp = cfg.fingerprint();
    assert_eq!(fp.branch_count, 1);
    assert!(fp.block_count > 1);

    assert!(cfg.blocks[0].uses.iter().any(|(n, _)| n == "x"));
}

#[test]
fn test_unreachable_code_after_return() {
    let source = "def unreachable_func():\n    return 1\n    x = 2\n    return x\n";
    let cfg = Cfg::from_source(source, "unreachable_func").expect("Should parse");
    let unreachable = cfg.find_unreachable_blocks();
    assert!(!unreachable.is_empty(), "Should find unreachable blocks");

    let mut found_x_block = false;
    for &block_id in &unreachable {
        let block = &cfg.blocks[block_id];
        if block.statements.iter().any(|s| s.line == 3) {
            found_x_block = true;
            break;
        }
    }
    assert!(found_x_block, "The block at line 3 should be unreachable");
}
