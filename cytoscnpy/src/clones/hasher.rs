//! LSH (Locality-Sensitive Hashing) for clone candidate pruning.
//!
//! Uses `MinHash` signatures to quickly find candidate clone pairs
//! without comparing every pair (O(n²) → O(n)).

use crate::clones::normalizer::NormalizedTree;
use rustc_hash::{FxHashMap, FxHashSet};
use std::hash::{Hash, Hasher};

/// LSH hasher for finding similar code blocks
#[derive(Debug, Clone)]
pub struct LshHasher {
    /// Number of bands
    num_bands: usize,
    /// Rows per band
    rows_per_band: usize,
    /// Total signature size = `num_bands` * `rows_per_band`
    signature_size: usize,
    /// Buckets with at least this many members are treated as boilerplate
    /// and skipped entirely to avoid O(n^2) blowups on common patterns.
    /// Raising this catches more real widespread duplication at the cost
    /// of more pairwise similarity comparisons downstream.
    boilerplate_threshold: usize,
    /// Hard cap on the number of candidate pairs returned. Prevents
    /// unbounded memory/time on pathological inputs; candidates beyond
    /// this cap are silently dropped.
    max_candidates: usize,
}

impl LshHasher {
    /// Create a new LSH hasher
    ///
    /// - `num_bands`: More bands = higher recall (more candidates)
    /// - `rows_per_band`: More rows = higher precision (fewer false positives)
    #[must_use]
    pub fn new(num_bands: usize, rows_per_band: usize) -> Self {
        assert!(num_bands > 0, "LSH bands must be positive");
        assert!(rows_per_band > 0, "LSH rows must be positive");
        assert!(
            num_bands.checked_mul(rows_per_band).is_some(),
            "LSH signature size overflow"
        );
        Self {
            num_bands,
            rows_per_band,
            signature_size: num_bands * rows_per_band,
            boilerplate_threshold: crate::constants::BOILERPLATE_THRESHOLD,
            max_candidates: 500_000,
        }
    }

    /// Override the boilerplate-bucket skip threshold and candidate pair
    /// cap. Both raise recall (fewer missed clones) at the cost of more
    /// downstream similarity comparisons.
    #[must_use]
    pub fn with_limits(mut self, boilerplate_threshold: usize, max_candidates: usize) -> Self {
        assert!(
            boilerplate_threshold >= 2,
            "LSH bucket limit must be at least 2"
        );
        assert!(max_candidates > 0, "LSH candidate limit must be positive");
        self.boilerplate_threshold = boilerplate_threshold;
        self.max_candidates = max_candidates;
        self
    }

    /// Generate `MinHash` signature for a normalized tree
    #[must_use]
    pub fn signature(&self, tree: &NormalizedTree) -> Vec<u64> {
        let shingles = Self::generate_shingles(tree);
        self.minhash(&shingles)
    }

    /// Find candidate pairs from a collection of trees.
    #[cfg(test)]
    #[must_use]
    pub fn find_candidates(&self, trees: &[NormalizedTree]) -> Vec<(usize, usize)> {
        let signatures: Vec<Vec<u64>> = trees.iter().map(|t| self.signature(t)).collect();
        self.find_candidates_from_signatures(&signatures)
    }

    /// Find candidate pairs from fingerprints (pre-computed signatures)
    #[must_use]
    pub fn find_candidates_from_fingerprints(
        &self,
        fingerprints: &[crate::clones::parser::CloneFingerprint],
    ) -> Vec<(usize, usize)> {
        // Bucket by band hashes directly from pre-computed signatures
        let mut buckets: FxHashMap<(usize, u64), Vec<usize>> = FxHashMap::default();

        for (idx, fp) in fingerprints.iter().enumerate() {
            for band in 0..self.num_bands {
                let band_hash = self.band_hash(&fp.lsh_signature, band);
                buckets.entry((band, band_hash)).or_default().push(idx);
            }
        }

        self.collect_pairs_from_buckets(&buckets)
    }

    /// Internal helper to find candidates from signatures.
    #[cfg(test)]
    fn find_candidates_from_signatures(&self, signatures: &[Vec<u64>]) -> Vec<(usize, usize)> {
        let mut buckets: FxHashMap<(usize, u64), Vec<usize>> = FxHashMap::default();

        for (idx, sig) in signatures.iter().enumerate() {
            for band in 0..self.num_bands {
                let band_hash = self.band_hash(sig, band);
                buckets.entry((band, band_hash)).or_default().push(idx);
            }
        }

        self.collect_pairs_from_buckets(&buckets)
    }

    /// Collect unique pairs from buckets
    fn collect_pairs_from_buckets(
        &self,
        buckets: &FxHashMap<(usize, u64), Vec<usize>>,
    ) -> Vec<(usize, usize)> {
        let mut candidates: FxHashSet<(usize, usize)> = FxHashSet::default();
        let mut ordered_buckets: Vec<_> = buckets.iter().collect();
        ordered_buckets.sort_unstable_by_key(|(key, indices)| (indices.len(), **key));

        // Give every member of every bucket at least one comparison before
        // spending the remaining budget on dense pair expansion. This keeps
        // large clone families visible instead of dropping the tail members.
        for (_, indices) in &ordered_buckets {
            for adjacent in indices.windows(2) {
                candidates.insert((adjacent[0].min(adjacent[1]), adjacent[0].max(adjacent[1])));
                if candidates.len() >= self.max_candidates {
                    return sorted_pairs(candidates);
                }
            }
        }

        for (_, indices) in ordered_buckets {
            // Very large boilerplate buckets are sampled deterministically
            // instead of disappearing completely at an arbitrary size cliff.
            let usable = indices
                .len()
                .min(self.boilerplate_threshold.saturating_sub(1));
            for i in 0..usable {
                for j in (i + 1)..usable {
                    let pair = (indices[i].min(indices[j]), indices[i].max(indices[j]));
                    candidates.insert(pair);
                    if candidates.len() >= self.max_candidates {
                        return sorted_pairs(candidates);
                    }
                }
            }
        }
        sorted_pairs(candidates)
    }

    /// Generate shingles (n-grams) from the tree structure
    fn generate_shingles(tree: &NormalizedTree) -> Vec<u64> {
        let kinds = tree.kind_sequence();
        if kinds.len() < 3 {
            // Too short, use individual kinds
            return kinds.iter().map(|k| hash_string(k)).collect();
        }

        // Generate 3-grams
        kinds.windows(3).map(hash_kind_window).collect()
    }

    /// Compute `MinHash` signature
    fn minhash(&self, shingles: &[u64]) -> Vec<u64> {
        if shingles.is_empty() {
            return vec![0; self.signature_size];
        }

        let mut signature = vec![u64::MAX; self.signature_size];

        for (i, slot) in signature.iter_mut().enumerate() {
            // Use different "hash functions" by combining with index
            for &shingle in shingles {
                let hash = hash_with_seed(shingle, i as u64);
                if hash < *slot {
                    *slot = hash;
                }
            }
        }

        signature
    }

    /// Compute hash for a single band
    fn band_hash(&self, signature: &[u64], band: usize) -> u64 {
        let start = band * self.rows_per_band;
        let end = (start + self.rows_per_band).min(signature.len());

        let mut hasher = rustc_hash::FxHasher::default();
        for slot in &signature[start..end] {
            slot.hash(&mut hasher);
        }
        hasher.finish()
    }
}

fn sorted_pairs(candidates: FxHashSet<(usize, usize)>) -> Vec<(usize, usize)> {
    let mut pairs: Vec<_> = candidates.into_iter().collect();
    pairs.sort_unstable();
    pairs
}

fn hash_kind_window(window: &[&str]) -> u64 {
    let mut hasher = rustc_hash::FxHasher::default();
    for kind in window {
        kind.hash(&mut hasher);
    }
    hasher.finish()
}

/// Hash a string
fn hash_string(s: &str) -> u64 {
    let mut hasher = rustc_hash::FxHasher::default();
    s.hash(&mut hasher);
    hasher.finish()
}

/// Hash with a seed (simulates different hash functions)
fn hash_with_seed(value: u64, seed: u64) -> u64 {
    let mut hasher = rustc_hash::FxHasher::default();
    value.hash(&mut hasher);
    seed.hash(&mut hasher);
    hasher.finish()
}

impl Default for LshHasher {
    fn default() -> Self {
        Self::new(20, 5) // 100 signature slots, 20 bands
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clones::normalizer::NormalizedNode;

    fn make_tree(kinds: &[&str]) -> NormalizedTree {
        NormalizedTree {
            nodes: kinds
                .iter()
                .map(|k| NormalizedNode {
                    kind: (*k).to_owned(),
                    label: None,
                    children: vec![],
                })
                .collect(),
        }
    }

    #[test]
    fn test_identical_trees_are_candidates() {
        let hasher = LshHasher::default();
        let trees = vec![
            make_tree(&["if", "assign", "return"]),
            make_tree(&["if", "assign", "return"]), // Identical
            make_tree(&["for", "call", "break"]),   // Different
        ];

        let candidates = hasher.find_candidates(&trees);
        assert!(candidates.contains(&(0, 1)));
        assert!(!candidates.contains(&(0, 2)));
    }

    #[test]
    fn test_similar_trees_may_be_candidates() {
        let hasher = LshHasher::default();
        let trees = vec![
            make_tree(&["if", "assign", "assign", "return"]),
            make_tree(&["if", "assign", "return"]), // Similar (missing one assign)
        ];

        let candidates = hasher.find_candidates(&trees);
        // May or may not match depending on hash functions, but shouldn't crash
        assert!(candidates.len() <= 1);
    }

    #[test]
    fn large_bucket_is_sampled_deterministically_instead_of_dropped() {
        let hasher = LshHasher::new(1, 1).with_limits(3, 10);
        let trees = vec![
            make_tree(&["if", "assign", "return"]),
            make_tree(&["if", "assign", "return"]),
            make_tree(&["if", "assign", "return"]),
        ];
        let first = hasher.find_candidates(&trees);
        let second = hasher.find_candidates(&trees);
        assert_eq!(first, second);
        assert_eq!(first, vec![(0, 1), (1, 2)]);
    }
}
