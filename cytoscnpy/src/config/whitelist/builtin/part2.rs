use super::super::{WhitelistEntry, WhitelistPattern};

pub(super) fn entries() -> Vec<WhitelistEntry> {
    vec![
        // string - formatter attributes
        WhitelistEntry {
            name: "parse".into(),
            category: Some("string".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "format_field".into(),
            category: Some("string".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "get_field".into(),
            category: Some("string".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "get_value".into(),
            category: Some("string".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "convert_field".into(),
            category: Some("string".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "format".into(),
            category: Some("string".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "vformat".into(),
            category: Some("string".into()),
            ..Default::default()
        },
        // sys - system attributes
        WhitelistEntry {
            name: "excepthook".into(),
            category: Some("sys".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "displayhook".into(),
            category: Some("sys".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "exitfunc".into(),
            category: Some("sys".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "stdin".into(),
            category: Some("sys".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "stdout".into(),
            category: Some("sys".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "stderr".into(),
            category: Some("sys".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "path".into(),
            category: Some("sys".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "modules".into(),
            category: Some("sys".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "meta_path".into(),
            category: Some("sys".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "path_hooks".into(),
            category: Some("sys".into()),
            ..Default::default()
        },
        // unittest - test methods
        WhitelistEntry {
            name: "setUp".into(),
            category: Some("unittest".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "tearDown".into(),
            category: Some("unittest".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "setUpClass".into(),
            category: Some("unittest".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "tearDownClass".into(),
            category: Some("unittest".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "setUpModule".into(),
            category: Some("unittest".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "tearDownModule".into(),
            category: Some("unittest".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "run".into(),
            category: Some("unittest".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "debug".into(),
            category: Some("unittest".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "countTestCases".into(),
            category: Some("unittest".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "defaultTestResult".into(),
            category: Some("unittest".into()),
            ..Default::default()
        },
        // collections - special methods
        WhitelistEntry {
            name: "__missing__".into(),
            category: Some("collections".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "_asdict".into(),
            category: Some("collections".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "_make".into(),
            category: Some("collections".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "_replace".into(),
            category: Some("collections".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "_fields".into(),
            category: Some("collections".into()),
            ..Default::default()
        },
        // ast - AST visitor methods
        WhitelistEntry {
            name: "visit".into(),
            category: Some("ast".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "generic_visit".into(),
            category: Some("ast".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "visit_*".into(),
            pattern: Some(WhitelistPattern::Wildcard),
            category: Some("ast".into()),
            ..Default::default()
        },
        // pint - physics units
        WhitelistEntry {
            name: "Quantity".into(),
            category: Some("pint".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "UnitRegistry".into(),
            category: Some("pint".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "Measurement".into(),
            category: Some("pint".into()),
            ..Default::default()
        },
        // Django-style patterns (common in web frameworks)
        WhitelistEntry {
            name: "Meta".into(),
            category: Some("django".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "Objects".into(),
            category: Some("django".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "DoesNotExist".into(),
            category: Some("django".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "MultipleObjectsReturned".into(),
            category: Some("django".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "save".into(),
            category: Some("django".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "delete".into(),
            category: Some("django".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "clean".into(),
            category: Some("django".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "validate_unique".into(),
            category: Some("django".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "get_absolute_url".into(),
            category: Some("django".into()),
            ..Default::default()
        },
    ]
}
