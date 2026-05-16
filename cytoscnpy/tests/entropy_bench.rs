//! Microbenchmark for entropy calculation. Run with: `cargo test --release --test entropy_bench -- --nocapture`.

use cytoscnpy::rules::secrets::calculate_entropy;
use std::time::Instant;

#[test]
fn bench_entropy_throughput() {
    let samples: Vec<String> = (0..200)
        .map(|i| {
            (0..64)
                .map(|j| char::from((b'A' + u8::try_from((i + j) % 62).unwrap_or(0)).min(b'z')))
                .collect::<String>()
        })
        .collect();
    let inputs: Vec<&str> = samples.iter().map(String::as_str).collect();

    // Warmup
    for s in &inputs {
        std::hint::black_box(calculate_entropy(s));
    }

    let iters = 100_000;
    let t = Instant::now();
    let mut acc = 0.0f64;
    for _ in 0..iters {
        for s in &inputs {
            acc += calculate_entropy(s);
        }
    }
    let elapsed = t.elapsed();
    std::hint::black_box(acc);

    let total_calls = iters * inputs.len();
    let ns_per_call = elapsed.as_nanos() as f64 / total_calls as f64;
    eprintln!(
        "entropy: {} calls in {:?} = {:.1} ns/call ({:.1} M calls/s)",
        total_calls,
        elapsed,
        ns_per_call,
        1_000.0 / ns_per_call
    );
}
