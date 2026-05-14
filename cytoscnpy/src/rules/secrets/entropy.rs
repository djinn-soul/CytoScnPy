/// Calculates Shannon entropy of a string over its bytes.
///
/// Secrets/tokens are effectively always ASCII, so byte-frequency is equivalent
/// to char-frequency for the intended workload. Using a fixed `[u32; 256]`
/// frequency table avoids the `HashMap` allocation that dominated this function.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn calculate_entropy(s: &str) -> f64 {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return 0.0;
    }

    let mut counts = [0u32; 256];
    for &b in bytes {
        counts[b as usize] += 1;
    }

    let len = bytes.len() as f64;
    let mut entropy = 0.0;
    for &count in &counts {
        if count > 0 {
            let p = f64::from(count) / len;
            entropy -= p * p.log2();
        }
    }
    entropy
}

/// Checks if a string has high entropy (likely random/secret).
#[must_use]
pub fn is_high_entropy(s: &str, threshold: f64, min_length: usize) -> bool {
    if s.len() < min_length {
        return false;
    }
    calculate_entropy(s) >= threshold
}
