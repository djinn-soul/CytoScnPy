//! Unit and integration tests for module architecture and circular dependency detection.

use super::builder::build_architecture_graph;
use super::cycles::detect_cycles;
use super::layering::{find_bidirectional_group_deps, find_god_modules};
use super::resolver::{ModuleResolver, ResolvedTarget};
use super::types::{ImportOccurrence, ModuleEdge, ModuleNode};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn make_node(id: usize, name: &str, group: &str) -> ModuleNode {
    ModuleNode {
        id,
        name: name.to_owned(),
        file_path: PathBuf::from(format!("{name}.py")),
        relative_path: format!("{name}.py"),
        line_count: 50,
        is_init: name.ends_with("__init__"),
        package_group: group.to_owned(),
    }
}

fn make_edge(from_id: usize, to_id: usize, nodes: &[ModuleNode], line: usize) -> ModuleEdge {
    ModuleEdge {
        from_id,
        to_id,
        from_name: nodes[from_id].name.clone(),
        to_name: nodes[to_id].name.clone(),
        occurrences: vec![ImportOccurrence {
            line,
            column: 1,
            is_type_checking: false,
            is_top_level: true,
            imported_symbol: None,
        }],
    }
}

#[test]
fn test_no_cycles_in_dag() {
    let nodes = vec![
        make_node(0, "app.a", "app"),
        make_node(1, "app.b", "app"),
        make_node(2, "app.c", "app"),
    ];
    let edges = vec![make_edge(0, 1, &nodes, 10), make_edge(1, 2, &nodes, 12)];

    let (count, largest, cycles) = detect_cycles(&nodes, &edges);
    assert_eq!(count, 0);
    assert_eq!(largest, 0);
    assert!(cycles.is_empty());
}

#[test]
fn test_direct_two_node_cycle() {
    let nodes = vec![make_node(0, "app.a", "app"), make_node(1, "app.b", "app")];
    let edges = vec![make_edge(0, 1, &nodes, 10), make_edge(1, 0, &nodes, 20)];

    let (count, largest, cycles) = detect_cycles(&nodes, &edges);
    assert_eq!(count, 1);
    assert_eq!(largest, 2);
    assert_eq!(cycles.len(), 1);
    assert!(cycles[0].has_runtime_impact);
    assert_eq!(cycles[0].modules.len(), 3); // A -> B -> A
    assert_eq!(cycles[0].modules[0], cycles[0].modules[2]);
}

#[test]
fn test_three_node_cycle() {
    let nodes = vec![
        make_node(0, "pkg.alpha", "pkg"),
        make_node(1, "pkg.beta", "pkg"),
        make_node(2, "pkg.gamma", "pkg"),
    ];
    let edges = vec![
        make_edge(0, 1, &nodes, 5),
        make_edge(1, 2, &nodes, 15),
        make_edge(2, 0, &nodes, 25),
    ];

    let (count, largest, cycles) = detect_cycles(&nodes, &edges);
    assert_eq!(count, 1);
    assert_eq!(largest, 3);
    assert_eq!(cycles[0].modules.len(), 4);
    assert_eq!(cycles[0].steps.len(), 3);
}

#[test]
fn test_god_module_detection_high_percentage() {
    let mut nodes = Vec::new();
    for i in 0..10 {
        nodes.push(make_node(i, &format!("pkg.mod{i}"), "pkg"));
    }

    let mut fan_in_map = HashMap::new();
    let mut fan_out_map = HashMap::new();

    // mod0 is imported by all other 9 modules (100% fan-in)
    fan_in_map.insert("pkg.mod0".to_owned(), 9);
    fan_out_map.insert("pkg.mod0".to_owned(), 1);

    let god_modules = find_god_modules(&nodes, &fan_in_map, &fan_out_map);
    assert_eq!(god_modules.len(), 1);
    assert_eq!(god_modules[0].module_name, "pkg.mod0");
    assert_eq!(god_modules[0].fan_in, 9);
}

#[test]
fn test_bidirectional_package_dependencies() {
    let nodes = vec![
        make_node(0, "auth.login", "auth"),
        make_node(1, "users.service", "users"),
    ];
    let edges = vec![
        make_edge(0, 1, &nodes, 10), // auth -> users
        make_edge(1, 0, &nodes, 20), // users -> auth
    ];

    let bi_deps = find_bidirectional_group_deps(&nodes, &edges);
    assert_eq!(bi_deps.len(), 1);
    assert!(
        (bi_deps[0].group_a == "auth" && bi_deps[0].group_b == "users")
            || (bi_deps[0].group_a == "users" && bi_deps[0].group_b == "auth")
    );
    assert_eq!(bi_deps[0].edges_a_to_b, 1);
    assert_eq!(bi_deps[0].edges_b_to_a, 1);
}

#[test]
fn test_end_to_end_project_graph() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let pkg_dir = root.join("my_pkg");
    fs::create_dir_all(&pkg_dir).unwrap();

    let file_a = pkg_dir.join("a.py");
    let file_b = pkg_dir.join("b.py");
    let file_init = pkg_dir.join("__init__.py");

    fs::write(
        &file_a,
        "from .b import helper\nimport requests\n\ndef run():\n    return helper()\n",
    )
    .unwrap();

    fs::write(
        &file_b,
        "from my_pkg.a import run\n\ndef helper():\n    return 42\n",
    )
    .unwrap();

    fs::write(&file_init, "# package init\n").unwrap();

    let files = vec![file_init, file_a, file_b];
    let roots = vec![root.to_path_buf()];

    let result = build_architecture_graph(&files, &roots);

    assert_eq!(result.stats.total_modules, 3);
    assert!(result.stats.total_internal_edges >= 2);
    assert_eq!(result.stats.circular_dependency_count, 1);
    assert_eq!(result.stats.largest_cycle_size, 2);
    assert!(result.external_imports.contains_key("requests"));
}

#[test]
fn test_module_resolver_relative_levels() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let p1 = root.join("pkg").join("a.py");
    let p2 = root.join("pkg").join("b.py");
    fs::create_dir_all(root.join("pkg")).unwrap();
    fs::write(&p1, "x = 1\n").unwrap();
    fs::write(&p2, "y = 2\n").unwrap();

    let resolver = ModuleResolver::build(&[p1, p2], &[root.to_path_buf()]);

    let id_a = *resolver.module_to_id.get("pkg.a").unwrap();
    let id_b = *resolver.module_to_id.get("pkg.b").unwrap();

    // From pkg.a: `from . import b`
    let target = resolver.resolve_import(id_a, None, Some("b"), 1);
    assert_eq!(target, Some(ResolvedTarget::Internal(id_b)));

    // From pkg.a: `from .b import helper`
    let target2 = resolver.resolve_import(id_a, Some("b"), Some("helper"), 1);
    assert_eq!(target2, Some(ResolvedTarget::Internal(id_b)));

    // From pkg.a: `import external_lib`
    let target3 = resolver.resolve_import(id_a, Some("external_lib"), None, 0);
    assert_eq!(
        target3,
        Some(ResolvedTarget::External("external_lib".to_owned()))
    );
}

#[test]
fn test_resolver_file_arguments_detect_cycles() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let dir = root.join("cycle_pkg");
    fs::create_dir_all(&dir).unwrap();

    let file_a = dir.join("a.py");
    let file_b = dir.join("b.py");

    fs::write(&file_a, "import b\n").unwrap();
    fs::write(&file_b, "import a\n").unwrap();

    // Passing the files directly as roots (as CLI positional arguments would)
    let files = vec![file_a.clone(), file_b.clone()];
    let roots = vec![file_a, file_b];

    let result = build_architecture_graph(&files, &roots);

    // Both files should resolve to their module names 'a' and 'b', NOT '__main__'
    assert_eq!(result.stats.total_modules, 2);
    let names: Vec<&str> = result.nodes.iter().map(|n| n.name.as_str()).collect();
    assert!(names.contains(&"a"));
    assert!(names.contains(&"b"));
    assert!(!names.contains(&"__main__"));

    // Circular import cycle between a and b must be detected
    assert_eq!(result.stats.circular_dependency_count, 1);
    assert_eq!(result.stats.largest_cycle_size, 2);
}

#[test]
fn test_resolver_package_directory_scan_preserves_package_and_detects_cycles() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let pkg = root.join("my_package");
    fs::create_dir_all(&pkg).unwrap();
    fs::write(pkg.join("__init__.py"), "").unwrap();

    let file_a = pkg.join("a.py");
    let file_b = pkg.join("b.py");

    fs::write(
        &file_a,
        "from my_package.b import b_fn\ndef a_fn(): return b_fn()\n",
    )
    .unwrap();
    fs::write(
        &file_b,
        "from my_package.a import a_fn\ndef b_fn(): return a_fn()\n",
    )
    .unwrap();

    // Scanning the package directory directly (roots = [pkg])
    let files = vec![pkg.join("__init__.py"), file_a, file_b];
    let roots = vec![pkg];

    let result = build_architecture_graph(&files, &roots);

    // Modules should retain canonical names "my_package.a" and "my_package.b"
    let names: Vec<&str> = result.nodes.iter().map(|n| n.name.as_str()).collect();
    assert!(names.contains(&"my_package.a"));
    assert!(names.contains(&"my_package.b"));

    // Cycle must be detected
    assert_eq!(result.stats.circular_dependency_count, 1);
    assert_eq!(result.stats.largest_cycle_size, 2);
}
