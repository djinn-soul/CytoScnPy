use super::*;
use std::fs;
use tempfile::{tempdir, TempDir};

fn write_file(name: &str, content: &str) -> (TempDir, std::path::PathBuf) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join(name);
    fs::write(&path, content).expect("write");
    (dir, path)
}

fn summarize(deps: &[DeclaredDependency]) -> Vec<(&str, bool, bool)> {
    deps.iter()
        .map(|dep| (dep.package_name.as_str(), dep.is_dev, dep.is_optional))
        .collect()
}

#[test]
fn setup_py_install_requires_extras_and_test_requirements() {
    let (_dir, path) = write_file(
        "setup.py",
        "from setuptools import setup\n\
         setup(\n\
             name='proj',\n\
             install_requires=['requests>=2.0', 'flask'],\n\
             tests_require=('pytest',),\n\
             extras_require={'docs': ['sphinx'], 'all': ['rich']},\n\
         )\n",
    );

    let deps = parse_setup_py(&path);
    let mut found = summarize(&deps);
    found.sort_unstable();

    assert_eq!(
        found,
        vec![
            ("flask", false, false),
            ("pytest", true, false),
            ("requests", false, false),
            ("rich", false, true),
            ("sphinx", false, true),
        ]
    );
    assert_eq!(
        deps[0].source,
        DependencySource::Setup("setup.py".to_owned())
    );
}

#[test]
fn setup_py_qualified_call_and_nested_position_are_found() {
    let (_dir, path) = write_file(
        "setup.py",
        "import setuptools\n\
         def main():\n    \
             setuptools.setup(install_requires=['requests'])\n\
         if __name__ == '__main__':\n    main()\n",
    );

    assert_eq!(
        summarize(&parse_setup_py(&path)),
        vec![("requests", false, false)]
    );
}

#[test]
fn setup_py_non_literal_requirements_are_skipped_not_guessed() {
    let (_dir, path) = write_file(
        "setup.py",
        "from setuptools import setup\n\
         reqs = open('requirements.txt').read().splitlines()\n\
         setup(install_requires=reqs, extras_require={'x': reqs})\n",
    );

    assert!(
        parse_setup_py(&path).is_empty(),
        "a computed requirement list cannot be recovered without executing the file"
    );
}

#[test]
fn setup_py_that_does_not_parse_yields_nothing() {
    let (_dir, path) = write_file("setup.py", "setup(install_requires=[");
    assert!(parse_setup_py(&path).is_empty());
    assert!(parse_setup_py(std::path::Path::new("no/such/setup.py")).is_empty());
}

#[test]
fn setup_cfg_indented_and_inline_requirement_lists() {
    let (_dir, path) = write_file(
        "setup.cfg",
        "[metadata]\nname = proj\ninstall_requires = ignored\n\n\
         [options]\n\
         python_requires = >=3.10\n\
         install_requires =\n    \
             requests>=2.0  # http\n    \
             flask\n\n\
         [options.extras_require]\n\
         docs = sphinx, myst-parser\n\
         test =\n    pytest\n",
    );

    let deps = parse_setup_cfg(&path);
    let mut found = summarize(&deps);
    found.sort_unstable();

    assert_eq!(
        found,
        vec![
            ("flask", false, false),
            ("myst-parser", false, true),
            ("pytest", false, true),
            ("requests", false, false),
            ("sphinx", false, true),
        ],
        "`install_requires` outside [options] and `python_requires` are not dependencies"
    );
}

#[test]
fn setup_cfg_that_declares_nothing_yields_nothing() {
    let (_dir, path) = write_file("setup.cfg", "[metadata]\nname = proj\n");
    assert!(parse_setup_cfg(&path).is_empty());
    assert!(parse_setup_cfg(std::path::Path::new("no/such/setup.cfg")).is_empty());
}
