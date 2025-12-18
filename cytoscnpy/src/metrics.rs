use std::f64;

/// Computes the Maintainability Index (MI).
///
/// Formula:
/// MI = 171 - 5.2 * ln(V) - 0.23 * G - 16.2 * ln(LOC)
///
/// Where:
/// - V = Halstead Volume
/// - G = Cyclomatic Complexity
/// - LOC = Lines of Code (SLOC)
///
/// If `comments` is provided (and > 0), it adds a comment weight:
/// MI = MI + 50 * sin(sqrt(2.4 * (comments / LOC)))
///
/// The result is clamped to [0, 100].
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn mi_compute(volume: f64, complexity: usize, sloc: usize, comments: usize) -> f64 {
    let mut mi = 171.0;

    // Halstead Volume
    if volume > 0.0 {
        mi -= 5.2 * volume.ln();
    }

    // Cyclomatic Complexity
    mi -= 0.23 * (complexity as f64);

    // SLOC
    if sloc > 0 {
        mi -= 16.2 * (sloc as f64).ln();
    }

    // Comment weight
    if comments > 0 && sloc > 0 {
        let per_comment = comments as f64 / sloc as f64;
        mi += 50.0 * (2.4 * per_comment).sqrt().sin();
    }

    // Clamp to 0-100
    mi.clamp(0.0, 100.0)
}

/// Ranks the Maintainability Index.
///
/// A: 20 - 100
/// B: 10 - 19
/// C: 0 - 9
#[must_use]
pub fn mi_rank(score: f64) -> char {
    if score >= 20.0 {
        'A'
    } else if score >= 10.0 {
        'B'
    } else {
        'C'
    }
}

/// Ranks the Cyclomatic Complexity.
///
/// A: 1 - 5
/// B: 6 - 10
/// C: 11 - 20
/// D: 21 - 30
/// E: 31 - 40
/// F: 41+
#[must_use]
pub fn cc_rank(cc: usize) -> char {
    if cc <= 5 {
        'A'
    } else if cc <= 10 {
        'B'
    } else if cc <= 20 {
        'C'
    } else if cc <= 30 {
        'D'
    } else if cc <= 40 {
        'E'
    } else {
        'F'
    }
}
