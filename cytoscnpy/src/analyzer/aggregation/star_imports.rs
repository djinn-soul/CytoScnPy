use super::state::AggregationState;
use rustc_hash::{FxHashMap, FxHashSet};

impl AggregationState {
    /// Resolves `from x import *` using explicit `__all__` when present and Python's
    /// public-name fallback otherwise. Repeated propagation supports star-import chains.
    pub(super) fn apply_star_import_bindings(&mut self) {
        let mut public_names = self.collect_public_top_level_names();
        let explicit_exports: FxHashMap<&str, &[String]> = self
            .all_module_exports
            .iter()
            .map(|(module, names)| (module.as_str(), names.as_slice()))
            .collect();

        loop {
            let mut changed = false;
            for (importer, source) in &self.all_star_imports {
                let names: Vec<String> = explicit_exports.get(source.as_str()).map_or_else(
                    || {
                        public_names
                            .get(source)
                            .into_iter()
                            .flatten()
                            .cloned()
                            .collect()
                    },
                    |exports| exports.to_vec(),
                );

                for name in names {
                    self.all_import_bindings
                        .entry(format!("{importer}.{name}"))
                        .or_insert_with(|| format!("{source}.{name}"));

                    if !name.starts_with('_') {
                        changed |= public_names
                            .entry(importer.clone())
                            .or_default()
                            .insert(name);
                    }
                }
            }

            if !changed {
                break;
            }
        }
    }

    fn collect_public_top_level_names(&self) -> FxHashMap<String, FxHashSet<String>> {
        let source_modules: FxHashSet<&str> = self
            .all_star_imports
            .iter()
            .map(|(_, source)| source.as_str())
            .collect();
        let mut names: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();

        for source in source_modules {
            let prefix = format!("{source}.");
            for definition in &self.all_defs {
                let Some(local_name) = definition.full_name.strip_prefix(&prefix) else {
                    continue;
                };
                if !local_name.contains('.') && !local_name.starts_with('_') {
                    names
                        .entry(source.to_owned())
                        .or_default()
                        .insert(local_name.to_owned());
                }
            }
        }

        names
    }
}
