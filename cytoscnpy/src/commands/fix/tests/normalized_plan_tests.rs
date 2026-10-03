use crate::commands::fix::apply_plan::{normalize_planned_edits, PlannedEdit};

fn edit(name: &str, start: usize, end: usize, replacement: Option<&str>) -> PlannedEdit {
    PlannedEdit {
        start_byte: start,
        end_byte: end,
        replacement: replacement.map(str::to_owned),
        name: name.to_owned(),
        removed_names: vec![name.to_owned()],
        item_type: "function",
        line: 1,
    }
}

#[test]
fn enclosing_deletion_counts_definitions_it_removes() {
    let planned = normalize_planned_edits(vec![
        edit("child", 5, 10, Some("_")),
        edit("parent", 0, 20, None),
    ])
    .unwrap();
    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].removed_names, ["parent", "child"]);
}

#[test]
fn conflicting_edits_fail_instead_of_reporting_success() {
    for edits in [
        vec![edit("first", 0, 10, None), edit("second", 5, 15, None)],
        vec![edit("first", 0, 10, Some("_")), edit("second", 5, 8, None)],
    ] {
        let error = normalize_planned_edits(edits).err().unwrap();
        assert!(error.to_string().contains("Conflicting fixes"));
        assert!(error.to_string().contains("first"));
        assert!(error.to_string().contains("second"));
    }
}
