use super::NotebookSource;

fn source() -> NotebookSource {
    NotebookSource::from_cells(vec![
        (1, "import os\nx = 1".to_owned()),
        (3, "y = 2\n".to_owned()),
    ])
}

#[test]
fn joins_cells_and_maps_offsets_back() {
    let notebook = source();
    assert_eq!(notebook.code, "import os\nx = 1\n\ny = 2\n");
    let second = notebook.cell_containing(17, 18).unwrap();
    assert_eq!((second.index, second.start, second.start_line), (3, 17, 4));
    assert_eq!(notebook.cell_source(3), Some("y = 2\n"));
    // The separator belongs to no cell.
    assert!(notebook.cell_containing(15, 17).is_none());
    assert_eq!(notebook.cell_at_line(2).map(|cell| cell.index), Some(1));
    assert_eq!(notebook.cell_at_line(4).map(|cell| cell.index), Some(3));
    // The blank separator line and line 0 belong to no cell.
    assert!(notebook.cell_at_line(3).is_none());
    assert!(notebook.cell_at_line(0).is_none());
}

#[test]
fn applies_edits_per_cell() {
    let notebook = source();
    let cells = notebook
        .apply_edits([(10, 11, "_"), (17, 18, "_")])
        .unwrap();
    assert_eq!(
        cells,
        vec![
            (1, "import os\n_ = 1".to_owned()),
            (3, "_ = 2\n".to_owned())
        ]
    );
    assert!(notebook.apply_edits([(10, 20, "")]).is_err());
}
