//! Pytest fixture metadata collection and cross-file resolution.

use crate::test_utils::TestAwareVisitor;
use crate::visitor::CytoScnPyVisitor;
use rustc_hash::{FxHashMap, FxHashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FixtureDefinitionRecord {
    pub(crate) full_name: String,
    pub(crate) fixture_name: String,
    pub(crate) file: PathBuf,
    pub(crate) module_name: String,
    pub(crate) is_conftest: bool,
    pub(crate) conftest_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FixtureRequestRecord {
    pub(crate) requested_name: String,
    pub(crate) requester_file: PathBuf,
    pub(crate) requester_module: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FixtureImportBinding {
    pub(crate) local_name: String,
    pub(crate) source_module: String,
    pub(crate) source_symbol: String,
    pub(crate) level: u32,
    pub(crate) requester_file: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PytestPluginDeclaration {
    pub(crate) declaring_file: PathBuf,
    pub(crate) plugin_module: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct FileFixtureMetadata {
    pub(crate) fixture_definitions: Vec<FixtureDefinitionRecord>,
    pub(crate) fixture_requests: Vec<FixtureRequestRecord>,
    pub(crate) fixture_imports: Vec<FixtureImportBinding>,
    pub(crate) pytest_plugins: Vec<PytestPluginDeclaration>,
}
#[must_use]
pub(crate) fn collect_file_fixture_metadata(
    visitor: &CytoScnPyVisitor<'_>,
    test_visitor: &TestAwareVisitor<'_>,
    file_path: &Path,
    module_name: &str,
) -> FileFixtureMetadata {
    let mut metadata = FileFixtureMetadata::default();
    let is_conftest = file_path.file_name().is_some_and(|n| n == "conftest.py");
    let conftest_dir = is_conftest.then(|| file_path.parent().unwrap_or(file_path).to_path_buf());

    let mut function_lookup: FxHashMap<(usize, String), String> = FxHashMap::default();
    for def in &visitor.definitions {
        if def.def_type == "function" {
            function_lookup.insert((def.line, def.simple_name.clone()), def.full_name.clone());
        }
    }

    let mut seen_fixture_defs = FxHashSet::default();
    for hint in &test_visitor.fixture_definitions {
        let key = (hint.line, hint.function_name.clone());
        let Some(full_name) = function_lookup.get(&key) else {
            continue;
        };
        if !seen_fixture_defs.insert(full_name.clone()) {
            continue;
        }
        metadata.fixture_definitions.push(FixtureDefinitionRecord {
            full_name: full_name.clone(),
            fixture_name: hint.fixture_name.clone(),
            file: file_path.to_path_buf(),
            module_name: module_name.to_owned(),
            is_conftest,
            conftest_dir: conftest_dir.clone(),
        });
    }

    for request in &test_visitor.fixture_request_names {
        if request.is_empty() {
            continue;
        }
        metadata.fixture_requests.push(FixtureRequestRecord {
            requested_name: request.clone(),
            requester_file: file_path.to_path_buf(),
            requester_module: module_name.to_owned(),
        });
    }

    for import in &test_visitor.fixture_imports {
        if import.local_name.is_empty() || import.source_symbol.is_empty() {
            continue;
        }
        let Some(source_module) =
            resolve_import_source_module(module_name, &import.source_module, import.level)
        else {
            continue;
        };
        metadata.fixture_imports.push(FixtureImportBinding {
            local_name: import.local_name.clone(),
            source_module,
            source_symbol: import.source_symbol.clone(),
            level: import.level,
            requester_file: file_path.to_path_buf(),
        });
    }

    for plugin in &test_visitor.pytest_plugins {
        if plugin.is_empty() {
            continue;
        }
        metadata.pytest_plugins.push(PytestPluginDeclaration {
            declaring_file: file_path.to_path_buf(),
            plugin_module: plugin.clone(),
        });
    }

    metadata
}
#[derive(Clone, Copy)]
struct CandidateMeta {
    rank: u8,
    conftest_depth: usize,
}
#[must_use]
pub(crate) fn resolve_fixture_reference_increments(
    fixture_definitions: &[FixtureDefinitionRecord],
    fixture_requests: &[FixtureRequestRecord],
    fixture_imports: &[FixtureImportBinding],
    pytest_plugins: &[PytestPluginDeclaration],
) -> FxHashMap<String, usize> {
    let mut defs_by_fixture_name: FxHashMap<String, Vec<usize>> = FxHashMap::default();
    let mut defs_by_module_fixture: FxHashMap<(String, String), Vec<usize>> = FxHashMap::default();
    let mut defs_by_module_function: FxHashMap<(String, String), Vec<usize>> = FxHashMap::default();

    for (idx, def) in fixture_definitions.iter().enumerate() {
        defs_by_fixture_name
            .entry(def.fixture_name.clone())
            .or_default()
            .push(idx);

        defs_by_module_fixture
            .entry((def.module_name.clone(), def.fixture_name.clone()))
            .or_default()
            .push(idx);

        let function_name = def
            .full_name
            .rsplit('.')
            .next()
            .unwrap_or(def.full_name.as_str())
            .to_owned();
        defs_by_module_function
            .entry((def.module_name.clone(), function_name))
            .or_default()
            .push(idx);
    }

    let mut imports_by_requester_local: FxHashMap<(PathBuf, String), Vec<usize>> =
        FxHashMap::default();
    for (idx, binding) in fixture_imports.iter().enumerate() {
        imports_by_requester_local
            .entry((binding.requester_file.clone(), binding.local_name.clone()))
            .or_default()
            .push(idx);
    }

    let mut increments: FxHashMap<String, usize> = FxHashMap::default();

    for request in fixture_requests {
        let mut best: Option<(usize, CandidateMeta)> = None;
        let mut consider_candidate = |idx: usize, rank: u8, conftest_depth: usize| {
            let new_meta = CandidateMeta {
                rank,
                conftest_depth,
            };
            let Some((old_idx, old_meta)) = best else {
                best = Some((idx, new_meta));
                return;
            };
            if should_replace_candidate(old_idx, old_meta, idx, new_meta, fixture_definitions) {
                best = Some((idx, new_meta));
            }
        };

        if let Some(candidate_indices) = defs_by_fixture_name.get(&request.requested_name) {
            for &idx in candidate_indices {
                if fixture_definitions[idx].file == request.requester_file {
                    consider_candidate(idx, 0, 0);
                }
            }

            for &idx in candidate_indices {
                let def = &fixture_definitions[idx];
                if !def.is_conftest {
                    continue;
                }
                let Some(conftest_dir) = &def.conftest_dir else {
                    continue;
                };
                if request.requester_file.starts_with(conftest_dir) {
                    consider_candidate(idx, 1, path_depth(conftest_dir));
                }
            }
        }

        if let Some(binding_indices) = imports_by_requester_local.get(&(
            request.requester_file.clone(),
            request.requested_name.clone(),
        )) {
            for &binding_idx in binding_indices {
                let binding = &fixture_imports[binding_idx];
                if let Some(indices) = defs_by_module_fixture
                    .get(&(binding.source_module.clone(), binding.source_symbol.clone()))
                {
                    for &idx in indices {
                        consider_candidate(idx, 2, 0);
                    }
                }
                if let Some(indices) = defs_by_module_function
                    .get(&(binding.source_module.clone(), binding.source_symbol.clone()))
                {
                    for &idx in indices {
                        consider_candidate(idx, 2, 0);
                    }
                }
            }
        }

        for plugin_module in visible_plugin_modules(&request.requester_file, pytest_plugins) {
            if let Some(indices) =
                defs_by_module_fixture.get(&(plugin_module, request.requested_name.clone()))
            {
                for &idx in indices {
                    consider_candidate(idx, 3, 0);
                }
            }
        }

        if let Some((best_idx, _)) = best {
            let full_name = fixture_definitions[best_idx].full_name.clone();
            *increments.entry(full_name).or_insert(0) += 1;
        }
    }

    increments
}

fn should_replace_candidate(
    old_idx: usize,
    old_meta: CandidateMeta,
    new_idx: usize,
    new_meta: CandidateMeta,
    defs: &[FixtureDefinitionRecord],
) -> bool {
    if new_meta.rank != old_meta.rank {
        return new_meta.rank < old_meta.rank;
    }
    if new_meta.rank == 1 && new_meta.conftest_depth != old_meta.conftest_depth {
        return new_meta.conftest_depth > old_meta.conftest_depth;
    }
    defs[new_idx].full_name < defs[old_idx].full_name
}

fn visible_plugin_modules(
    requester_file: &Path,
    pytest_plugins: &[PytestPluginDeclaration],
) -> FxHashSet<String> {
    let mut visible = FxHashSet::default();
    for plugin in pytest_plugins {
        if plugin_visible_to_request(plugin, requester_file) {
            visible.insert(plugin.plugin_module.clone());
        }
    }
    visible
}

fn plugin_visible_to_request(plugin: &PytestPluginDeclaration, requester_file: &Path) -> bool {
    if plugin
        .declaring_file
        .file_name()
        .is_some_and(|name| name == "conftest.py")
    {
        if let Some(parent) = plugin.declaring_file.parent() {
            return requester_file.starts_with(parent);
        }
        return false;
    }
    requester_file == plugin.declaring_file
}

fn path_depth(path: &Path) -> usize {
    path.components().count()
}

fn resolve_import_source_module(
    requester_module: &str,
    source_module: &str,
    level: u32,
) -> Option<String> {
    if level == 0 {
        return Some(source_module.to_owned());
    }

    let mut package_parts: Vec<&str> = requester_module.split('.').collect();
    package_parts.pop();

    let ascend = usize::try_from(level - 1).ok()?;
    if ascend > package_parts.len() {
        return None;
    }
    package_parts.truncate(package_parts.len().saturating_sub(ascend));

    if !source_module.is_empty() {
        package_parts.extend(source_module.split('.'));
    }

    Some(package_parts.join("."))
}
