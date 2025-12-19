//! LSH (Locality-Sensitive Hashing) for clone candidate pruning.
//!
//! Uses `MinHash` signatures to quickly find candidate clone pairs
//! without comparing every pair (O(n²) → O(n)).

use crate::clones::normalizer::NormalizedTree;
use rustc_hash::FxHashMap;
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
}

impl LshHasher {
    /// Create a new LSH hasher
    ///
    /// - `num_bands`: More bands = higher recall (more candidates)
    /// - `rows_per_band`: More rows = higher precision (fewer false positives)
    #[must_use]
    pub fn new(num_bands: usize, rows_per_band: usize) -> Self {
        Self {
            num_bands,
            rows_per_band,
            signature_size: num_bands * rows_per_band,
        }
    }

    /// Generate `MinHash` signature for a normalized tree
    #[must_use]
    pub fn signature(&self, tree: &NormalizedTree) -> Vec<u64> {
        let shingles = self.generate_shingles(tree);
        self.minhash(&shingles)
    }

    /// Find candidate pairs from a collection of trees
    #[must_use]
    pub fn find_candidates(&self, trees: &[NormalizedTree]) -> Vec<(usize, usize)> {
        let signatures: Vec<Vec<u64>> = trees.iter().map(|t| self.signature(t)).collect();

        // Bucket by band hashes
        let mut buckets: FxHashMap<(usize, u64), Vec<usize>> = FxHashMap::default();

        for (idx, sig) in signatures.iter().enumerate() {
            for band in 0..self.num_bands {
                let band_hash = self.band_hash(sig, band);
                buckets.entry((band, band_hash)).or_default().push(idx);
            }
        }

        // Collect pairs that share any bucket
        let mut candidates: FxHashMap<(usize, usize), ()> = FxHashMap::default();
        for indices in buckets.values() {
            if indices.len() > 1 {
                for i in 0..indices.len() {
                    for j in (i + 1)..indices.len() {
                        let pair = if indices[i] < indices[j] {
                            (indices[i], indices[j])
                        } else {
                            (indices[j], indices[i])
                        };
                        candidates.insert(pair, ());
                    }
                }
            }
        }

        candidates.into_keys().collect()
    }

    /// Generate shingles (n-grams) from the tree structure
    fn generate_shingles(&self, tree: &NormalizedTree) -> Vec<u64> {
        let kinds = tree.kind_sequence();
        if kinds.len() < 3 {
            // Too short, use individual kinds
            return kinds.iter().map(|k| hash_string(k)).collect();
        }

        // Generate 3-grams
        kinds
            .windows(3)
            .map(|window| {
                let combined = format!("{}-{}-{}", window[0], window[1], window[2]);
                hash_string(&combined)
            })
            .collect()
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
        for i in start..end {
            signature[i].hash(&mut hasher);
        }
        hasher.finish()
    }
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
                    kind: (*k).to_string(),
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
}
