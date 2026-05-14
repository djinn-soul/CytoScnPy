//! Microbench for the `add_ref`-style `HashMap` insertion pattern.
//! Compares `entry(key.to_owned())` (current) vs check-then-insert (proposed)
//! to confirm whether avoiding the per-call `to_owned` on the existing-key
//! path is worth the extra hash on the missing-key path.
//!
//! Run with: `cargo test --release --test add_ref_bench -- --nocapture`

use rustc_hash::FxHashMap;
use std::time::Instant;

fn add_ref_current(map: &mut FxHashMap<String, usize>, name: &str) {
    *map.entry(name.to_owned()).or_insert(0) += 1;
}

fn add_ref_checked(map: &mut FxHashMap<String, usize>, name: &str) {
    if let Some(c) = map.get_mut(name) {
        *c += 1;
    } else {
        map.insert(name.to_owned(), 1);
    }
}

fn workload() -> Vec<String> {
    // Roughly mimics the visitor: a small pool of distinct symbols, each
    // referenced many times. Real files have ~50-500 distinct refs, each
    // hit on average a handful of times.
    let mut pool: Vec<String> = (0..256)
        .map(|i| format!("mymod.submod.symbol_{i}"))
        .collect();
    // Repeat each key ~16x in random-ish order
    let mut seq = Vec::with_capacity(pool.len() * 16);
    for r in 0..16 {
        for (i, k) in pool.iter().enumerate() {
            let _ = r;
            let _ = i;
            seq.push(k.clone());
        }
    }
    pool.clear();
    seq
}

#[test]
fn bench_add_ref_variants() {
    let seq = workload();
    let iters = 1_000;

    // Warmup
    for _ in 0..10 {
        let mut m: FxHashMap<String, usize> = FxHashMap::default();
        for s in &seq {
            add_ref_current(&mut m, s);
        }
        std::hint::black_box(&m);
    }

    let t = Instant::now();
    let mut total_current = 0;
    for _ in 0..iters {
        let mut m: FxHashMap<String, usize> = FxHashMap::default();
        for s in &seq {
            add_ref_current(&mut m, s);
        }
        total_current += m.len();
    }
    let dt_current = t.elapsed();
    std::hint::black_box(total_current);

    let t = Instant::now();
    let mut total_checked = 0;
    for _ in 0..iters {
        let mut m: FxHashMap<String, usize> = FxHashMap::default();
        for s in &seq {
            add_ref_checked(&mut m, s);
        }
        total_checked += m.len();
    }
    let dt_checked = t.elapsed();
    std::hint::black_box(total_checked);

    let calls = iters * seq.len();
    eprintln!(
        "workload: {} calls, {} distinct keys",
        calls,
        seq.len() / 16
    );
    eprintln!(
        "current  (entry+to_owned): {:?} = {:.1} ns/call",
        dt_current,
        dt_current.as_nanos() as f64 / calls as f64
    );
    eprintln!(
        "checked  (get_mut/else):   {:?} = {:.1} ns/call",
        dt_checked,
        dt_checked.as_nanos() as f64 / calls as f64
    );
    eprintln!(
        "speedup: {:.2}x",
        dt_current.as_nanos() as f64 / dt_checked.as_nanos() as f64
    );
    assert_eq!(total_current, total_checked, "results must match");
}
