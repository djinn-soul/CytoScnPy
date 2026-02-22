use super::super::WhitelistEntry;

pub(super) fn entries() -> Vec<WhitelistEntry> {
    vec![
        // Flask-style patterns
        WhitelistEntry {
            name: "before_request".into(),
            category: Some("flask".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "after_request".into(),
            category: Some("flask".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "teardown_request".into(),
            category: Some("flask".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "errorhandler".into(),
            category: Some("flask".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "context_processor".into(),
            category: Some("flask".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "url_value_preprocessor".into(),
            category: Some("flask".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "url_defaults".into(),
            category: Some("flask".into()),
            ..Default::default()
        },
        // Pytest fixtures and hooks (already covered by framework detection, but explicit here)
        WhitelistEntry {
            name: "pytest_configure".into(),
            category: Some("pytest".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "pytest_unconfigure".into(),
            category: Some("pytest".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "pytest_collection_modifyitems".into(),
            category: Some("pytest".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "pytest_addoption".into(),
            category: Some("pytest".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "pytest_generate_tests".into(),
            category: Some("pytest".into()),
            ..Default::default()
        },
        // Entry points and plugin patterns
        WhitelistEntry {
            name: "main".into(),
            category: Some("entry_point".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "setup".into(),
            category: Some("entry_point".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "teardown".into(),
            category: Some("entry_point".into()),
            ..Default::default()
        },
        // Magic methods that are called dynamically
        WhitelistEntry {
            name: "__call__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__getattr__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__setattr__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__delattr__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__getattribute__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__dir__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__len__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__iter__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__next__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__contains__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__bool__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__str__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__repr__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__hash__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__eq__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__ne__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__lt__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__le__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__gt__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__ge__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__getitem__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__setitem__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
        WhitelistEntry {
            name: "__delitem__".into(),
            category: Some("magic".into()),
            ..Default::default()
        },
    ]
}
