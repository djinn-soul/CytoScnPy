//! Interval union algorithms for non-overlapping line calculations.

/// Merges overlapping or adjacent 1-indexed line intervals `(start, end)` into disjoint intervals.
#[must_use]
pub fn merge_intervals(mut intervals: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    if intervals.is_empty() {
        return Vec::new();
    }

    // Sort by start line ascending, then end line descending
    intervals.sort_unstable_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)));

    let mut merged = Vec::with_capacity(intervals.len());
    let (mut cur_start, mut cur_end) = intervals[0];

    for &(start, end) in &intervals[1..] {
        if start <= cur_end + 1 {
            // Overlapping or adjacent interval: extend current boundary
            cur_end = cur_end.max(end);
        } else {
            // Non-overlapping interval: push current and start new
            merged.push((cur_start, cur_end));
            cur_start = start;
            cur_end = end;
        }
    }

    merged.push((cur_start, cur_end));
    merged
}

/// Calculates the total number of unique (non-overlapping) lines covered by the intervals.
#[must_use]
pub fn count_non_overlapping_lines(intervals: Vec<(usize, usize)>) -> usize {
    merge_intervals(intervals)
        .into_iter()
        .map(|(start, end)| end.saturating_sub(start) + 1)
        .sum()
}
