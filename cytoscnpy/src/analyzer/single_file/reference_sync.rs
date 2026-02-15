use crate::analyzer::CytoScnPy;
#[cfg(feature = "cfg")]
use crate::cfg::flow::analyze_reaching_definitions;
#[cfg(feature = "cfg")]
use crate::cfg::Cfg;
use crate::visitor::Definition;
use rustc_hash::{FxHashMap, FxHashSet};

impl CytoScnPy {
    pub(super) fn sync_definition_references(
        definitions: &mut [Definition],
        ref_counts: &FxHashMap<String, usize>,
    ) {
        let mut def_type_map: FxHashMap<String, String> = FxHashMap::default();
        let mut simple_name_counts: FxHashMap<String, usize> = FxHashMap::default();

        for def in definitions.iter() {
            def_type_map.insert(def.full_name.clone(), def.def_type.clone());
            *simple_name_counts
                .entry(def.simple_name.clone())
                .or_insert(0) += 1;
        }

        for def in definitions.iter_mut() {
            let mut current_refs = 0;
            let is_unique = simple_name_counts
                .get(&def.simple_name)
                .copied()
                .unwrap_or(0)
                == 1;

            if let Some(count) = ref_counts.get(&def.full_name) {
                current_refs = *count;
            }

            if current_refs == 0 {
                let mut fallback_refs = 0;

                if is_unique && !def.is_enum_member {
                    if let Some(count) = ref_counts.get(&def.simple_name) {
                        fallback_refs += *count;
                    }
                }

                let is_attribute_like = match def.def_type.as_str() {
                    "method" | "class" | "class_variable" => true,
                    "variable" | "parameter" => {
                        if let Some((parent, _)) = def.full_name.rsplit_once('.') {
                            def_type_map.get(parent).is_some_and(|kind| kind == "class")
                        } else {
                            false
                        }
                    }
                    _ => false,
                };

                if is_attribute_like {
                    if let Some(count) = ref_counts.get(&format!(".{}", def.simple_name)) {
                        fallback_refs += *count;
                    }
                }

                if def.is_enum_member {
                    if let Some(dot_idx) = def.full_name.rfind('.') {
                        let parent = &def.full_name[..dot_idx];
                        if let Some(class_dot) = parent.rfind('.') {
                            let class_member =
                                format!("{}.{}", &parent[class_dot + 1..], def.simple_name);
                            if let Some(count) = ref_counts.get(&class_member) {
                                fallback_refs = fallback_refs.max(*count);
                            }
                        }
                    }
                }

                if fallback_refs > 0 {
                    current_refs = fallback_refs;
                }
            }

            def.references = current_refs;
        }
    }

    pub(super) fn mark_captured_definitions(
        definitions: &mut [Definition],
        captured_definitions: &FxHashSet<String>,
    ) {
        for def in definitions.iter_mut() {
            if captured_definitions.contains(&def.full_name) {
                def.is_captured = true;
                def.references += 1;
            }
        }
    }

    #[cfg(feature = "cfg")]
    pub(super) fn refine_flow_sensitive(
        source: &str,
        definitions: &mut [Definition],
        dynamic_scopes: &FxHashSet<String>,
    ) {
        let mut function_scopes: FxHashMap<String, (usize, usize)> = FxHashMap::default();
        for def in definitions.iter() {
            if def.def_type == "function" || def.def_type == "method" {
                function_scopes.insert(def.full_name.clone(), (def.line, def.end_line));
            }
        }

        for (func_name, (start_line, end_line)) in function_scopes {
            let simple_name = func_name.split('.').next_back().unwrap_or(&func_name);
            if dynamic_scopes.contains(&func_name) || dynamic_scopes.contains(simple_name) {
                continue;
            }

            let lines: Vec<&str> = source
                .lines()
                .skip(start_line.saturating_sub(1))
                .take(end_line.saturating_sub(start_line) + 1)
                .collect();
            let func_source = lines.join("\n");

            if let Some(cfg) = Cfg::from_source(&func_source, simple_name) {
                let flow_results = analyze_reaching_definitions(&cfg);
                for def in definitions.iter_mut() {
                    if (def.def_type == "variable" || def.def_type == "parameter")
                        && def.full_name.starts_with(&func_name)
                    {
                        let relative_name = &def.full_name[func_name.len()..];
                        if let Some(var_key) = relative_name.strip_prefix('.') {
                            let rel_line = def.line.saturating_sub(start_line) + 1;
                            let is_used = flow_results.is_def_used(&cfg, var_key, rel_line);
                            if !is_used && def.references > 0 && !def.is_captured {
                                def.references = 0;
                            }
                        }
                    }
                }
            }
        }
    }
}
