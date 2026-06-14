use super::{classify_method, first_parameter_name, visitor::LcomVisitor, MethodKind};
use ruff_python_ast::{self as ast, Stmt};
use std::collections::{HashMap, HashSet};

pub(super) struct LcomMethodData {
    methods: HashSet<String>,
    method_usage: HashMap<String, HashSet<String>>,
    method_calls: HashMap<String, HashSet<String>>,
}

impl LcomMethodData {
    fn new() -> Self {
        Self {
            methods: HashSet::new(),
            method_usage: HashMap::new(),
            method_calls: HashMap::new(),
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.methods.is_empty()
    }
}

pub(super) fn collect_methods(class_body: &[Stmt]) -> LcomMethodData {
    let mut data = LcomMethodData::new();
    for stmt in class_body {
        let Stmt::FunctionDef(func) = stmt else {
            continue;
        };
        collect_method(func, &mut data);
    }
    data
}

fn collect_method(func: &ast::StmtFunctionDef, data: &mut LcomMethodData) {
    let method_name = func.name.id.as_str();
    if is_dunder_method(method_name) {
        return;
    }

    let method_kind = classify_method(&func.decorator_list);
    if matches!(method_kind, MethodKind::Static) {
        return;
    }

    let method_name = method_name.to_owned();
    let receiver_name = receiver_name_for_method(func, method_kind);
    let mut visitor = LcomVisitor::new(receiver_name);
    visitor.visit_body(&func.body);

    data.methods.insert(method_name.clone());
    data.method_usage
        .insert(method_name.clone(), visitor.used_fields);
    data.method_calls
        .insert(method_name, visitor.called_methods);
}

fn is_dunder_method(method_name: &str) -> bool {
    method_name.starts_with("__") && method_name.ends_with("__")
}

fn receiver_name_for_method(
    func: &ast::StmtFunctionDef,
    method_kind: MethodKind,
) -> Option<String> {
    match method_kind {
        MethodKind::Instance | MethodKind::Class => first_parameter_name(&func.parameters),
        MethodKind::Static => None,
    }
}

pub(super) fn build_adjacency(
    data: &LcomMethodData,
) -> (Vec<String>, HashMap<String, Vec<String>>) {
    let method_list: Vec<String> = data.methods.iter().cloned().collect();
    let mut adjacency = initial_adjacency(&method_list);

    for i in 0..method_list.len() {
        connect_method_pair(i, &method_list, data, &mut adjacency);
    }

    (method_list, adjacency)
}

fn initial_adjacency(method_list: &[String]) -> HashMap<String, Vec<String>> {
    let mut adjacency = HashMap::new();
    for method in method_list {
        adjacency.insert(method.clone(), Vec::new());
    }
    adjacency
}

fn connect_method_pair(
    i: usize,
    method_list: &[String],
    data: &LcomMethodData,
    adjacency: &mut HashMap<String, Vec<String>>,
) {
    for j in (i + 1)..method_list.len() {
        let first = &method_list[i];
        let second = &method_list[j];
        if methods_are_connected(first, second, data) {
            add_edge(first, second, adjacency);
        }
    }
}

fn methods_are_connected(first: &str, second: &str, data: &LcomMethodData) -> bool {
    share_fields(first, second, data) || call_each_other(first, second, data)
}

fn share_fields(first: &str, second: &str, data: &LcomMethodData) -> bool {
    let Some(first_fields) = data.method_usage.get(first) else {
        return false;
    };
    let Some(second_fields) = data.method_usage.get(second) else {
        return false;
    };
    first_fields.intersection(second_fields).next().is_some()
}

fn call_each_other(first: &str, second: &str, data: &LcomMethodData) -> bool {
    let Some(first_calls) = data.method_calls.get(first) else {
        return false;
    };
    let Some(second_calls) = data.method_calls.get(second) else {
        return false;
    };
    first_calls.contains(second) || second_calls.contains(first)
}

fn add_edge(first: &str, second: &str, adjacency: &mut HashMap<String, Vec<String>>) {
    if let Some(neighbors) = adjacency.get_mut(first) {
        neighbors.push(second.to_owned());
    }
    if let Some(neighbors) = adjacency.get_mut(second) {
        neighbors.push(first.to_owned());
    }
}

pub(super) fn count_connected_components(
    method_list: &[String],
    adjacency: &HashMap<String, Vec<String>>,
) -> usize {
    let mut visited = HashSet::new();
    let mut components = 0;

    for method in method_list {
        if visited.contains(method) {
            continue;
        }
        components += 1;
        visit_component(method, adjacency, &mut visited);
    }

    components
}

fn visit_component(
    start: &str,
    adjacency: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
) {
    let mut stack = vec![start.to_owned()];
    visited.insert(start.to_owned());

    while let Some(current) = stack.pop() {
        push_unvisited_neighbors(&current, adjacency, visited, &mut stack);
    }
}

fn push_unvisited_neighbors(
    current: &str,
    adjacency: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    stack: &mut Vec<String>,
) {
    let Some(neighbors) = adjacency.get(current) else {
        return;
    };
    for neighbor in neighbors {
        if visited.insert(neighbor.clone()) {
            stack.push(neighbor.clone());
        }
    }
}
