/// Calculates Shannon entropy of a string.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn calculate_entropy(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }

    let mut char_counts: std::collections::HashMap<char, usize> = std::collections::HashMap::new();
    let len = s.len() as f64;

    for c in s.chars() {
        *char_counts.entry(c).or_insert(0) += 1;
    }

    char_counts
        .values()
        .map(|&count| {
            let p = count as f64 / len;
            -p * p.log2()
        })
        .sum()
}

/// Checks if a string has high entropy (likely random/secret).
#[must_use]
pub fn is_high_entropy(s: &str, threshold: f64, min_length: usize) -> bool {
    if s.len() < min_length {
        return false;
    }
    calculate_entropy(s) >= threshold
}
