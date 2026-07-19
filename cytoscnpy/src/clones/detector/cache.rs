use crate::clones::normalizer::NormalizedTree;
use crate::clones::parser;
use crate::clones::{CloneType, Normalizer};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;

/// Maximum source files retained while comparing candidate pairs.
const MAX_PAIR_CACHE_FILES: usize = 64;

pub(super) struct PreparedSubtree {
    pub(super) subtree: parser::Subtree,
    pub(super) raw_tree: NormalizedTree,
    pub(super) id_tree: NormalizedTree,
}

/// Bounded FIFO cache for parsed and normalized candidate files.
#[derive(Default)]
pub(super) struct PairSubtreeCache {
    entries: HashMap<PathBuf, Vec<PreparedSubtree>>,
    insertion_order: VecDeque<PathBuf>,
}

impl PairSubtreeCache {
    pub(super) fn load(&mut self, file: &PathBuf, min_lines: usize) {
        if self.entries.contains_key(file) {
            return;
        }
        if self.entries.len() == MAX_PAIR_CACHE_FILES {
            if let Some(oldest) = self.insertion_order.pop_front() {
                self.entries.remove(&oldest);
            }
        }
        self.insertion_order.push_back(file.clone());
        self.entries
            .insert(file.clone(), prepare_subtrees(file, min_lines));
    }

    pub(super) fn find(&self, file: &PathBuf, start_byte: usize) -> Option<&PreparedSubtree> {
        self.entries.get(file).and_then(|subtrees| {
            subtrees
                .iter()
                .find(|subtree| subtree.subtree.start_byte == start_byte)
        })
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }
}

fn prepare_subtrees(file: &PathBuf, min_lines: usize) -> Vec<PreparedSubtree> {
    std::fs::read_to_string(file)
        .ok()
        .and_then(|source| parser::extract_subtrees_with_min_lines(&source, file, min_lines).ok())
        .map_or_else(Vec::new, |subtrees| {
            let raw_normalizer = Normalizer::for_clone_type(CloneType::Type1);
            let id_normalizer = Normalizer::for_clone_type(CloneType::Type2);
            subtrees
                .into_iter()
                .map(|subtree| PreparedSubtree {
                    raw_tree: raw_normalizer.normalize(&subtree),
                    id_tree: id_normalizer.normalize(&subtree),
                    subtree,
                })
                .collect()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caches_failed_load_as_empty() {
        let file = std::env::temp_dir().join(format!(
            "cytoscnpy-missing-clone-source-{}",
            std::process::id()
        ));
        let mut cache = PairSubtreeCache {
            entries: HashMap::new(),
            insertion_order: VecDeque::new(),
        };
        cache.load(&file, 1);
        assert!(cache.find(&file, 0).is_none());
    }

    #[test]
    fn evicts_oldest_files_at_the_capacity_limit() {
        let mut cache = PairSubtreeCache::default();
        for index in 0..=MAX_PAIR_CACHE_FILES {
            cache.load(&PathBuf::from(format!("missing-{index}.py")), 1);
        }
        assert_eq!(cache.len(), MAX_PAIR_CACHE_FILES);
        assert!(cache.find(&PathBuf::from("missing-0.py"), 0).is_none());
    }
}
