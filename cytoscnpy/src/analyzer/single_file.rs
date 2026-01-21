//! Single file analysis logic.

use super::{AnalysisResult, AnalysisSummary, CytoScnPy, ParseError};
#[cfg(feature = "cfg")]
use crate::cfg::flow::analyze_reaching_definitions;
#[cfg(feature = "cfg")]
use crate::cfg::Cfg;
use crate::framework::FrameworkAwareVisitor;
use crate::halstead::{analyze_halstead, HalsteadMetrics};
use crate::metrics::mi_compute;
use crate::raw_metrics::{analyze_raw, RawMetrics};
use crate::rules::secrets::{scan_secrets, SecretFinding};
use crate::rules::Finding;
use crate::test_utils::TestAwareVisitor;
use crate::utils::LineIndex;
use crate::visitor::{CytoScnPyVisitor, Definition};

use ruff_python_parser::parse_module;
use rustc_hash::{FxHashMap, FxHashSet};
use std::fs;
use std::path::Path;

use super::traversal::{collect_docstring_lines, convert_byte_range_to_line};
use super::{apply_heuristics, apply_penalties};

impl CytoScnPy {
    /// Processes a single file (from disk or notebook) and returns analysis results.
    /// Used by the directory traversal for high-performance scanning.
    #[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
    #[must_use]
    pub fn process_single_file(
        &self,
        file_path: &Path,
        root_path: &Path,
    ) -> (
        Vec<Definition>,
        FxHashMap<String, usize>,
        FxHashMap<String, FxHashSet<String>>, // Protocol methods
        Vec<SecretFinding>,
        Vec<Finding>,
        Vec<Finding>,
        Vec<ParseError>,
        usize,
        RawMetrics,
        HalsteadMetrics,
        f64,
        f64,
        usize, // File size in bytes
    ) {
        let is_notebook = file_path.extension().is_some_and(|e| e == "ipynb");

        if let Some(delay_ms) = self.debug_delay_ms {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        }

        let mut file_complexity = 0.0;
        let mut file_mi = 0.0;

        let source = if is_notebook {
            match crate::ipynb::extract_notebook_code(file_path, Some(&self.analysis_root)) {
                Ok(code) => code,
                Err(e) => {
                    return (
                        Vec::new(),
                        FxHashMap::default(),
                        FxHashMap::default(), // protocol methods
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        vec![ParseError {
                            file: file_path.to_path_buf(),
                            error: format!("Failed to parse notebook: {e}"),
                        }],
                        0,
                        RawMetrics::default(),
                        HalsteadMetrics::default(),
                        0.0,
                        0.0,
                        0,
                    );
                }
            }
        } else {
            match fs::read_to_string(file_path) {
                Ok(code) => code,
                Err(e) => {
                    return (
                        Vec::new(),
                        FxHashMap::default(),
                        FxHashMap::default(), // protocol methods
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        vec![ParseError {
                            file: file_path.to_path_buf(),
                            error: format!("Failed to read file: {e}"),
                        }],
                        0,
                        RawMetrics::default(),
                        HalsteadMetrics::default(),
                        0.0,
                        0.0,
                        0,
                    );
                }
            }
        };

        let file_size = source.len();
        let line_index = LineIndex::new(&source);
        let ignored_lines = crate::utils::get_ignored_lines(&source);

        let relative_path = file_path.strip_prefix(root_path).unwrap_or(file_path);
        let components: Vec<&str> = relative_path
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();

        let mut module_parts = Vec::new();
        for (i, part) in components.iter().enumerate() {
            if i == components.len() - 1 {
                if let Some(stem) = Path::new(part).file_stem() {
                    let stem_str = stem.to_string_lossy();
                    if stem_str != "__init__" {
                        module_parts.push(stem_str.to_string());
                    }
                }
            } else {
                module_parts.push((*part).to_owned());
            }
        }
        let module_name = module_parts.join(".");

        let mut visitor =
            CytoScnPyVisitor::new(file_path.to_path_buf(), module_name.clone(), &line_index);
        let mut framework_visitor = FrameworkAwareVisitor::new(&line_index);
        let mut test_visitor = TestAwareVisitor::new(file_path, &line_index);

        let mut secrets = Vec::new();
        let mut danger = Vec::new();
        let mut quality = Vec::new();
        let mut parse_errors = Vec::new();

        match parse_module(&source) {
            Ok(parsed) => {
                let module = parsed.into_syntax();

                let mut docstring_lines = rustc_hash::FxHashSet::default();
                if self.enable_secrets && self.config.cytoscnpy.secrets_config.skip_docstrings {
                    collect_docstring_lines(&module.body, &line_index, &mut docstring_lines, 0);
                }

                if self.enable_secrets {
                    secrets = scan_secrets(
                        &source,
                        &file_path.to_path_buf(),
                        &self.config.cytoscnpy.secrets_config,
                        Some(&docstring_lines),
                    );
                }

                let entry_point_calls = crate::entry_point::detect_entry_point_calls(&module.body);

                for stmt in &module.body {
                    framework_visitor.visit_stmt(stmt);
                    test_visitor.visit_stmt(stmt);
                    visitor.visit_stmt(stmt);
                }

                for call_name in &entry_point_calls {
                    visitor.add_ref(call_name.clone());
                    if !module_name.is_empty() {
                        let qualified = format!("{module_name}.{call_name}");
                        visitor.add_ref(qualified);
                    }
                }

                for fw_ref in &framework_visitor.framework_references {
                    visitor.add_ref(fw_ref.clone());
                    if !module_name.is_empty() {
                        let qualified = format!("{module_name}.{fw_ref}");
                        visitor.add_ref(qualified);
                    }
                }

                let exports = visitor.exports.clone();
                for export_name in &exports {
                    visitor.add_ref(export_name.clone());
                    if !module_name.is_empty() {
                        let qualified = format!("{module_name}.{export_name}");
                        visitor.add_ref(qualified);
                    }
                }

                // 1. Synchronize reference counts from visitor's bag before refinement
                let ref_counts = visitor.references.clone();

                // Pre-compute simple name uniqueness to safely use fallback
                let mut simple_name_counts: rustc_hash::FxHashMap<String, usize> =
                    rustc_hash::FxHashMap::default();
                for def in &visitor.definitions {
                    *simple_name_counts
                        .entry(def.simple_name.clone())
                        .or_insert(0) += 1;
                }

                // Pre-compute full_name -> def_type for scope lookups.
                // These maps enable "safe" fallback strategies when exact full-name matching fails:
                // 1. Uniqueness (checked via simple_name_counts above) ensures we only fallback to
                //    simple names when they are unambiguous in the file.
                // 2. Context awareness (via def_type_map) ensures we only attempt attribute-style
                //    matches (.attr) when the parent scope is a class, avoiding invalid variable matches.
                let mut def_type_map: rustc_hash::FxHashMap<String, String> =
                    rustc_hash::FxHashMap::default();
                for def in &visitor.definitions {
                    def_type_map.insert(def.full_name.clone(), def.def_type.clone());
                }

                for def in &mut visitor.definitions {
                    let mut current_refs = 0;
                    let is_unique = simple_name_counts
                        .get(&def.simple_name)
                        .copied()
                        .unwrap_or(0)
                        == 1;

                    // 1. Try full qualified name match (Preferred)
                    if let Some(count) = ref_counts.get(&def.full_name) {
                        current_refs = *count;
                    }

                    // 2. Fallback strategies if not found
                    if current_refs == 0 {
                        let mut fallback_refs = 0;

                        // Strategy A: Simple name match (Variables/Imports)
                        // Only safe if the name is unique to avoid ambiguity
                        if is_unique && !def.is_enum_member {
                            if let Some(count) = ref_counts.get(&def.simple_name) {
                                fallback_refs += *count;
                            }
                        }

                        // Strategy B: Dot-prefixed attribute match (.attr)
                        // Used for methods and attributes where `obj.attr` is used but `obj` type is unknown.
                        // We check this ONLY if the definition is likely an attribute/method.
                        let is_attribute_like = match def.def_type.as_str() {
                            "method" | "class" | "class_variable" => true,
                            "variable" | "parameter" => {
                                // Check if parent scope is a Class
                                if let Some((parent, _)) = def.full_name.rsplit_once('.') {
                                    def_type_map.get(parent).is_some_and(|t| t == "class")
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

                        // Strategy C: Enum Member special fallback
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

                        // Apply fallback result
                        if fallback_refs > 0 {
                            current_refs = fallback_refs;
                        }
                    }

                    def.references = current_refs;
                }

                // 1.5. Populate is_captured and mark as used if captured
                for def in &mut visitor.definitions {
                    if visitor.captured_definitions.contains(&def.full_name) {
                        def.is_captured = true;
                        def.references += 1;
                    }
                }

                // 2. Dynamic code handling
                // 2. Dynamic code handling
                let any_dynamic = !visitor.dynamic_scopes.is_empty();
                let module_is_dynamic = visitor.dynamic_scopes.contains(&module_name);

                for def in &mut visitor.definitions {
                    // 1. Global eval affects everything (conservative)
                    if module_is_dynamic {
                        def.references += 1;
                        continue;
                    }

                    // 2. Any local eval affects module-level variables (globals)
                    // We explicitly SKIP secrets here (don't mark them as used).
                    // Why: Even if `eval` exists, a hardcoded secret identifying as "unused" is highly suspicious
                    // and should be reported to the user rather than suppressed by dynamic code heuristics.
                    if any_dynamic && !def.is_potential_secret {
                        if let Some(idx) = def.full_name.rfind('.') {
                            if def.full_name[..idx] == module_name {
                                def.references += 1;
                                continue;
                            }
                        }
                    }

                    // 3. Scoped eval usage (locals)
                    for scope in &visitor.dynamic_scopes {
                        if def.full_name.starts_with(scope) {
                            let scope_len = scope.len();
                            // Ensure boundary match
                            if def.full_name.len() > scope_len
                                && def.full_name.as_bytes()[scope_len] == b'.'
                            {
                                def.references += 1;
                                break;
                            }
                        }
                    }
                }

                // 3. Flow-sensitive refinement
                #[cfg(feature = "cfg")]
                Self::refine_flow_sensitive(
                    &source,
                    &mut visitor.definitions,
                    &visitor.dynamic_scopes,
                );

                // 3. Apply penalties and heuristics
                for def in &mut visitor.definitions {
                    apply_penalties(
                        def,
                        &framework_visitor,
                        &test_visitor,
                        &ignored_lines,
                        self.include_tests,
                    );
                    apply_heuristics(def);

                    if def.def_type == "method"
                        && visitor.self_referential_methods.contains(&def.full_name)
                    {
                        def.is_self_referential = true;
                    }
                }

                let mut rules = Vec::new();
                if self.enable_danger {
                    rules.extend(crate::rules::danger::get_danger_rules());
                }
                if self.enable_quality {
                    rules.extend(crate::rules::quality::get_quality_rules(&self.config));
                }

                if !rules.is_empty() {
                    let mut linter = crate::linter::LinterVisitor::new(
                        rules,
                        file_path.to_path_buf(),
                        line_index.clone(),
                        self.config.clone(),
                    );
                    for stmt in &module.body {
                        linter.visit_stmt(stmt);
                    }

                    for finding in linter.findings {
                        // Check for inline suppression (pragma: no cytoscnpy or pragma: no cytoscnpy(RULE_ID))
                        // This helper handles both broad suppressions (Suppress::All) and specific rule exclusions.
                        if crate::utils::is_line_suppressed(
                            &ignored_lines,
                            finding.line,
                            &finding.rule_id,
                        ) {
                            continue;
                        }
                        if finding.rule_id.starts_with("CSP-D") {
                            danger.push(finding);
                        } else if finding.rule_id.starts_with("CSP-Q")
                            || finding.rule_id.starts_with("CSP-L")
                            || finding.rule_id.starts_with("CSP-C")
                            || finding.category == "Best Practices"
                            || finding.category == "Maintainability"
                        {
                            // TODO: (Temporary fix) Route by category until Quality Rule IDs are finalized.
                            quality.push(finding);
                        }
                    }

                    // Apply taint analysis if enabled
                    if self.enable_danger
                        && self
                            .config
                            .cytoscnpy
                            .danger_config
                            .enable_taint
                            .unwrap_or(crate::constants::TAINT_ENABLED_DEFAULT)
                    {
                        use crate::rules::danger::taint_aware::TaintAwareDangerAnalyzer;
                        let custom_sources = self
                            .config
                            .cytoscnpy
                            .danger_config
                            .custom_sources
                            .clone()
                            .unwrap_or_default();
                        let custom_sinks = self
                            .config
                            .cytoscnpy
                            .danger_config
                            .custom_sinks
                            .clone()
                            .unwrap_or_default();
                        let taint_analyzer =
                            TaintAwareDangerAnalyzer::with_custom(custom_sources, custom_sinks);

                        let taint_context =
                            taint_analyzer.build_taint_context(&source, &file_path.to_path_buf());

                        // Update filtering logic: remove findings without taint
                        danger = TaintAwareDangerAnalyzer::filter_findings_with_taint(
                            danger,
                            &taint_context,
                        );

                        // Enhance severity for confirmed taint paths
                        TaintAwareDangerAnalyzer::enhance_severity_with_taint(
                            &mut danger,
                            &taint_context,
                        );
                    }

                    // Filter based on excluded_rules
                    if let Some(excluded) = &self.config.cytoscnpy.danger_config.excluded_rules {
                        danger.retain(|f| !excluded.contains(&f.rule_id));
                    }

                    // Filter based on severity_threshold
                    // This acts as a global suppression mechanism for lower-priority issues,
                    // allowing users to focus only on findings that meet a minimum risk level (e.g., HIGH+).
                    if let Some(threshold) = &self.config.cytoscnpy.danger_config.severity_threshold
                    {
                        let threshold_val = match threshold.to_uppercase().as_str() {
                            "CRITICAL" => 4,
                            "HIGH" => 3,
                            "MEDIUM" => 2,
                            "LOW" => 1,
                            _ => 0,
                        };
                        danger.retain(|f| {
                            let severity_val = match f.severity.to_uppercase().as_str() {
                                "CRITICAL" => 4,
                                "HIGH" => 3,
                                "MEDIUM" => 2,
                                "LOW" => 1,
                                _ => 0,
                            };
                            severity_val >= threshold_val
                        });
                    }
                }

                if self.enable_quality {
                    let raw = analyze_raw(&source);
                    let h_metrics = analyze_halstead(&ruff_python_ast::Mod::Module(module.clone()));
                    let volume = h_metrics.volume;
                    let complexity =
                        crate::complexity::calculate_module_complexity(&source).unwrap_or(1);
                    file_complexity = complexity as f64;
                    file_mi = mi_compute(volume, complexity, raw.sloc, raw.comments);

                    if let Some(min_mi) = self.config.cytoscnpy.min_mi {
                        if file_mi < min_mi {
                            quality.push(Finding {
                                message: format!(
                                    "Maintainability Index too low ({file_mi:.2} < {min_mi:.2})"
                                ),
                                rule_id: "CSP-Q303".to_owned(),
                                category: "Maintainability".to_owned(),
                                file: file_path.to_path_buf(),
                                line: 1,
                                col: 0,
                                severity: "HIGH".to_owned(),
                            });
                        }
                    }
                }
            }
            Err(e) => {
                if self.enable_secrets {
                    secrets = scan_secrets(
                        &source,
                        &file_path.to_path_buf(),
                        &self.config.cytoscnpy.secrets_config,
                        None,
                    );
                }
                let error_msg = format!("{e}");
                let readable_error = convert_byte_range_to_line(&error_msg, &source);
                parse_errors.push(ParseError {
                    file: file_path.to_path_buf(),
                    error: readable_error,
                });
            }
        }

        if let Some(ref pb) = self.progress_bar {
            pb.inc(1);
        }

        (
            visitor.definitions,
            visitor.references,
            visitor.protocol_methods,
            secrets,
            danger,
            quality,
            parse_errors,
            source.lines().count(),
            if self.enable_quality {
                analyze_raw(&source)
            } else {
                RawMetrics::default()
            },
            HalsteadMetrics::default(), // Computed later if needed
            file_complexity,
            file_mi,
            file_size,
        )
    }

    /// Analyzes a single string of code (mostly for testing).
    #[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
    #[must_use]
    pub fn analyze_code(&self, code: &str, file_path: &Path) -> AnalysisResult {
        let source = code.to_owned();
        let line_index = LineIndex::new(&source);
        let ignored_lines = crate::utils::get_ignored_lines(&source);

        let module_name = file_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Clone is necessary: Visitor takes ownership to build full names,
        // but we need `module_name` later for dynamic scope checks.
        let mut visitor =
            CytoScnPyVisitor::new(file_path.to_path_buf(), module_name.clone(), &line_index);
        let mut framework_visitor = FrameworkAwareVisitor::new(&line_index);
        let mut test_visitor = TestAwareVisitor::new(file_path, &line_index);

        let mut parse_errors = Vec::new();
        let mut secrets_res = Vec::new();
        let mut danger_res = Vec::new();
        let mut quality_res = Vec::new();

        match parse_module(&source) {
            Ok(parsed) => {
                let module = parsed.into_syntax();

                for stmt in &module.body {
                    framework_visitor.visit_stmt(stmt);
                    test_visitor.visit_stmt(stmt);
                    visitor.visit_stmt(stmt);
                }

                // Sync and Refine
                let ref_counts = visitor.references.clone();
                // Pre-compute maps
                let mut def_type_map: rustc_hash::FxHashMap<String, String> =
                    rustc_hash::FxHashMap::default();
                let mut simple_name_counts: rustc_hash::FxHashMap<String, usize> =
                    rustc_hash::FxHashMap::default();

                // Pre-compute maps for Safe Fallback Strategy:
                // 1. simple_name_counts: Ensures we only fallback to simple names if they are unique in the file/scope.
                // 2. def_type_map: Allows checking parent types (e.g. is this a class?) to validate attribute lookups.
                // These are critical for handling dynamic Python where clear imports might be missing.
                for def in &visitor.definitions {
                    def_type_map.insert(def.full_name.clone(), def.def_type.clone());
                    *simple_name_counts
                        .entry(def.simple_name.clone())
                        .or_insert(0) += 1;
                }

                for def in &mut visitor.definitions {
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
                                    def_type_map.get(parent).is_some_and(|t| t == "class")
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

                // 1.5. Populate is_captured and mark as used if captured
                for def in &mut visitor.definitions {
                    if visitor.captured_definitions.contains(&def.full_name) {
                        def.is_captured = true;
                        def.references += 1;
                    }
                }

                // 1.5. Dynamic code handling
                let any_dynamic = !visitor.dynamic_scopes.is_empty();
                let module_is_dynamic = visitor.dynamic_scopes.contains(&module_name);

                for def in &mut visitor.definitions {
                    if module_is_dynamic {
                        def.references += 1;
                        continue;
                    }
                    if any_dynamic {
                        if let Some(idx) = def.full_name.rfind('.') {
                            if def.full_name[..idx] == module_name {
                                def.references += 1;
                                continue;
                            }
                        }
                    }
                    for scope in &visitor.dynamic_scopes {
                        if def.full_name.starts_with(scope) {
                            let scope_len = scope.len();
                            if def.full_name.len() > scope_len
                                && def.full_name.as_bytes()[scope_len] == b'.'
                            {
                                def.references += 1;
                                break;
                            }
                        }
                    }
                }

                #[cfg(feature = "cfg")]
                Self::refine_flow_sensitive(
                    &source,
                    &mut visitor.definitions,
                    &visitor.dynamic_scopes,
                );

                for def in &mut visitor.definitions {
                    apply_penalties(
                        def,
                        &framework_visitor,
                        &test_visitor,
                        &ignored_lines,
                        self.include_tests,
                    );
                    apply_heuristics(def);
                }

                // --- Duck Typing Logic (same as aggregate_results) ---
                // 1. Map Class -> Method Names
                let mut class_methods: rustc_hash::FxHashMap<
                    String,
                    rustc_hash::FxHashSet<String>,
                > = rustc_hash::FxHashMap::default();
                for def in &visitor.definitions {
                    if def.def_type == "method" {
                        if let Some(parent) = def.full_name.rfind('.').map(|i| &def.full_name[..i])
                        {
                            class_methods
                                .entry(parent.to_owned())
                                .or_default()
                                .insert(def.simple_name.clone());
                        }
                    }
                }

                // 2. Identification of implicit implementations
                // Heuristic for Duck Typing / Implicit Interfaces:
                // We assume a class implements a protocol/interface if it matches a significant portion of its methods.
                // - Thresholds: At least 3 matching methods AND >= 70% overlap.
                // - Why: This reduces false positives where classes share 1-2 common method names (like "get" or "save")
                //   but aren't truly interchangeable, while correctly catching implementation-heavy patterns
                //   without explicit inheritance.
                let mut implicitly_used_methods: rustc_hash::FxHashSet<String> =
                    rustc_hash::FxHashSet::default();

                for (class_name, methods) in &class_methods {
                    for proto_methods in visitor.protocol_methods.values() {
                        let intersection_count = methods.intersection(proto_methods).count();
                        let proto_len = proto_methods.len();

                        if proto_len > 0 && intersection_count >= 3 {
                            let ratio = intersection_count as f64 / proto_len as f64;
                            if ratio >= 0.7 {
                                for method in methods.intersection(proto_methods) {
                                    implicitly_used_methods
                                        .insert(format!("{class_name}.{method}"));
                                }
                            }
                        }
                    }
                }

                // 3. Apply implicit usage
                for def in &mut visitor.definitions {
                    if implicitly_used_methods.contains(&def.full_name) {
                        def.references = std::cmp::max(def.references, 1);
                    }
                }

                if self.enable_secrets {
                    let mut docstring_lines = rustc_hash::FxHashSet::default();
                    if self.config.cytoscnpy.secrets_config.skip_docstrings {
                        collect_docstring_lines(&module.body, &line_index, &mut docstring_lines, 0);
                    }
                    secrets_res = scan_secrets(
                        &source,
                        &file_path.to_path_buf(),
                        &self.config.cytoscnpy.secrets_config,
                        Some(&docstring_lines),
                    );
                }

                // Run LinterVisitor with enabled rules.
                let mut rules = Vec::new();
                if self.enable_danger {
                    rules.extend(crate::rules::danger::get_danger_rules());
                }
                if self.enable_quality {
                    rules.extend(crate::rules::quality::get_quality_rules(&self.config));
                }

                if !rules.is_empty() {
                    let mut linter = crate::linter::LinterVisitor::new(
                        rules,
                        file_path.to_path_buf(),
                        line_index.clone(),
                        self.config.clone(),
                    );
                    for stmt in &module.body {
                        linter.visit_stmt(stmt);
                    }

                    for finding in linter.findings {
                        // Check for suppression (pragma: no cytoscnpy)
                        // This handles both blanket suppressions and rule-specific exclusions.
                        if let Some(suppression) = ignored_lines.get(&finding.line) {
                            match suppression {
                                crate::utils::Suppression::All => continue,
                                crate::utils::Suppression::Specific(rules) => {
                                    if rules.contains(&finding.rule_id) {
                                        continue;
                                    }
                                }
                            }
                        }

                        if finding.rule_id.starts_with("CSP-D") {
                            danger_res.push(finding);
                        } else if finding.rule_id.starts_with("CSP-Q")
                            || finding.rule_id.starts_with("CSP-L")
                            || finding.rule_id.starts_with("CSP-C")
                            || finding.category == "Best Practices"
                            || finding.category == "Maintainability"
                        {
                            // Route "Maintainability" issues (like low MI score) to the quality report.
                            // This ensures they are grouped with other code quality metrics rather than security vulnerabilities.
                            // TODO: (Temporary fix) Route by category until Quality Rule IDs are finalized.
                            quality_res.push(finding);
                        }
                    }

                    // Apply taint analysis if enabled
                    if self.enable_danger
                        && self
                            .config
                            .cytoscnpy
                            .danger_config
                            .enable_taint
                            .unwrap_or(crate::constants::TAINT_ENABLED_DEFAULT)
                    {
                        use crate::rules::danger::taint_aware::TaintAwareDangerAnalyzer;
                        let custom_sources = self
                            .config
                            .cytoscnpy
                            .danger_config
                            .custom_sources
                            .clone()
                            .unwrap_or_default();
                        let custom_sinks = self
                            .config
                            .cytoscnpy
                            .danger_config
                            .custom_sinks
                            .clone()
                            .unwrap_or_default();
                        let taint_analyzer =
                            TaintAwareDangerAnalyzer::with_custom(custom_sources, custom_sinks);

                        let taint_context =
                            taint_analyzer.build_taint_context(&source, &file_path.to_path_buf());

                        danger_res = TaintAwareDangerAnalyzer::filter_findings_with_taint(
                            danger_res,
                            &taint_context,
                        );

                        TaintAwareDangerAnalyzer::enhance_severity_with_taint(
                            &mut danger_res,
                            &taint_context,
                        );
                    }

                    // Filter based on excluded_rules
                    if let Some(excluded) = &self.config.cytoscnpy.danger_config.excluded_rules {
                        danger_res.retain(|f| !excluded.contains(&f.rule_id));
                    }

                    // Filter based on severity_threshold
                    if let Some(threshold) = &self.config.cytoscnpy.danger_config.severity_threshold
                    {
                        let threshold_val = match threshold.to_uppercase().as_str() {
                            "CRITICAL" => 4,
                            "HIGH" => 3,
                            "MEDIUM" => 2,
                            "LOW" => 1,
                            _ => 0,
                        };
                        danger_res.retain(|f| {
                            let severity_val = match f.severity.to_uppercase().as_str() {
                                "CRITICAL" => 4,
                                "HIGH" => 3,
                                "MEDIUM" => 2,
                                "LOW" => 1,
                                _ => 0,
                            };
                            severity_val >= threshold_val
                        });
                    }
                }
            }
            Err(e) => {
                if self.enable_secrets {
                    secrets_res = scan_secrets(
                        &source,
                        &file_path.to_path_buf(),
                        &self.config.cytoscnpy.secrets_config,
                        None,
                    );
                }
                parse_errors.push(ParseError {
                    file: file_path.to_path_buf(),
                    error: format!("{e}"),
                });
            }
        }

        // Final aggregation
        let total_definitions = visitor.definitions.len();
        let mut unused_functions = Vec::new();
        let mut unused_methods = Vec::new();
        let mut unused_classes = Vec::new();
        let mut unused_imports = Vec::new();
        let mut unused_variables = Vec::new();
        let mut unused_parameters = Vec::new();

        for def in visitor.definitions {
            if def.confidence >= self.confidence_threshold && def.references == 0 {
                // Check for valid suppression
                // Note: is_line_suppressed handles "no cytoscnpy" and specific rules if we supported them for variables
                // For now, we assume any suppression on the line applies to the unused variable finding
                if crate::utils::is_line_suppressed(&ignored_lines, def.line, "CSP-V001") {
                    continue;
                }

                match def.def_type.as_str() {
                    "function" => unused_functions.push(def),
                    "method" => unused_methods.push(def),
                    "class" => unused_classes.push(def),
                    "import" => unused_imports.push(def),
                    "variable" => unused_variables.push(def),
                    "parameter" => unused_parameters.push(def),
                    _ => {}
                }
            }
        }

        AnalysisResult {
            unused_functions,
            unused_methods,
            unused_imports,
            unused_classes,
            unused_variables,
            unused_parameters,
            secrets: secrets_res,
            danger: danger_res,
            quality: quality_res,
            taint_findings: Vec::new(),
            parse_errors,
            clones: Vec::new(),
            analysis_summary: AnalysisSummary {
                total_files: 1,
                total_lines_analyzed: source.lines().count(),
                total_definitions,
                ..AnalysisSummary::default()
            },
            file_metrics: Vec::new(),
        }
    }

    #[cfg(feature = "cfg")]
    fn refine_flow_sensitive(
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
