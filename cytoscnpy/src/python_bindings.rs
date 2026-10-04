//! Python bindings for the CytoScnPy analyzer.
//!
//! This module provides PyO3 bindings that expose Rust functionality to Python.
//! It creates the `cytoscnpy` Python module with `run`, `scan_json`, and `scan_code_json`.

use pyo3::types::PyModuleMethods;
use pyo3::{pyfunction, types::PyModule, wrap_pyfunction, Bound, PyErr, PyResult, Python};
use std::path::PathBuf;

mod signals;
use signals::SignalGuard;

/// Python-callable wrapper for the analyzer CLI.
///
/// This function accepts a list of command-line arguments and delegates to the
/// Rust implementation. It releases the Python GIL while running to allow
/// concurrent Python threads.
///
/// # Arguments
/// * `py` - Python interpreter token
/// * `args` - Command-line arguments as a vector of strings
///
/// # Returns
/// Exit code (0 for success, non-zero for errors)
///
/// # Examples
/// ```python
/// import cytoscnpy
/// exit_code = cytoscnpy.run(['--help'])
/// ```
#[pyfunction]
fn run(py: Python, args: Vec<String>) -> PyResult<i32> {
    // Reset cancellation flag
    crate::CANCELLED.store(false, std::sync::atomic::Ordering::SeqCst);

    let signal_guard = SignalGuard::new();

    // Release the GIL while running the Rust code
    let result = py.detach(|| {
        crate::entry_point::run_with_args(args)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    });

    drop(signal_guard);
    py.check_signals()?;

    // If cancelled, ensure we raise KeyboardInterrupt for Python to handle
    if crate::CANCELLED.load(std::sync::atomic::Ordering::Relaxed) {
        return Err(PyErr::new::<pyo3::exceptions::PyKeyboardInterrupt, _>(
            "Operation cancelled by user",
        ));
    }

    result
}

/// In-process static analysis scan returning raw JSON results.
#[pyfunction]
#[pyo3(signature = (
    paths = vec![".".to_owned()],
    confidence = None,
    secrets = None,
    danger = None,
    quality = None,
    include_tests = None,
    exclude_folders = Vec::new(),
    include_folders = Vec::new(),
    include_ipynb = None,
    clones = None,
    clone_similarity = None
))]
#[allow(clippy::too_many_arguments)]
fn scan_json(
    py: Python,
    paths: Vec<String>,
    confidence: Option<u8>,
    secrets: Option<bool>,
    danger: Option<bool>,
    quality: Option<bool>,
    include_tests: Option<bool>,
    exclude_folders: Vec<String>,
    include_folders: Vec<String>,
    include_ipynb: Option<bool>,
    clones: Option<bool>,
    clone_similarity: Option<f64>,
) -> PyResult<String> {
    crate::CANCELLED.store(false, std::sync::atomic::Ordering::SeqCst);

    let path_bufs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    for path in &path_bufs {
        if !path.exists() {
            return Err(PyErr::new::<pyo3::exceptions::PyFileNotFoundError, _>(
                format!("Path does not exist: {}", path.display()),
            ));
        }
    }

    let root = path_bufs.first().map_or_else(
        || PathBuf::from("."),
        |p| {
            if p.is_file() {
                p.parent()
                    .unwrap_or_else(|| std::path::Path::new("."))
                    .to_path_buf()
            } else {
                p.clone()
            }
        },
    );

    let config = crate::config::Config::try_load_from_path(&root).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Configuration error: {e}"))
    })?;

    let conf = confidence.or(config.cytoscnpy.confidence).unwrap_or(60);
    validate_confidence(conf)?;
    let sec = secrets.unwrap_or_else(|| {
        config.cytoscnpy.secrets.unwrap_or(false)
            || config.cytoscnpy.fail_on_secrets.unwrap_or(false)
    });
    let dan = danger.unwrap_or_else(|| {
        config.cytoscnpy.danger.unwrap_or(false) || config.cytoscnpy.fail_on_danger.unwrap_or(false)
    });
    let qua = quality.unwrap_or_else(|| {
        config.cytoscnpy.quality.unwrap_or(false)
            || config.cytoscnpy.min_mi.is_some()
            || config.cytoscnpy.max_complexity.is_some()
            || config.cytoscnpy.max_lines.is_some()
            || config.cytoscnpy.max_args.is_some()
            || config.cytoscnpy.max_nesting.is_some()
    });
    let inc_tests = include_tests
        .or(config.cytoscnpy.include_tests)
        .unwrap_or(false);
    let inc_ipynb = include_ipynb
        .or(config.cytoscnpy.include_ipynb)
        .unwrap_or(false);
    let run_clones = clones.or(config.cytoscnpy.clones).unwrap_or(false);
    let sim = clone_similarity
        .or(config.cytoscnpy.clone_similarity)
        .unwrap_or(0.8);

    let mut excludes = config.cytoscnpy.exclude_folders.clone().unwrap_or_default();
    excludes.extend(exclude_folders);

    let mut includes = config.cytoscnpy.include_folders.clone().unwrap_or_default();
    includes.extend(include_folders);

    let signal_guard = SignalGuard::new();

    let json_result = py.detach(move || {
        let mut analyzer = crate::analyzer::CytoScnPy::new(
            conf,
            sec,
            dan,
            qua,
            inc_tests,
            excludes.clone(),
            includes.clone(),
            inc_ipynb,
            false,
            config.clone(),
        )
        .with_root(root);

        if !config.cytoscnpy.whitelist.is_empty() {
            analyzer.whitelist_matcher =
                Some(crate::whitelist::WhitelistMatcher::with_user_entries(
                    config.cytoscnpy.whitelist.clone(),
                ));
        }

        let mut result = analyzer.analyze_paths(&path_bufs);

        if run_clones {
            let clone_options = crate::commands::CloneOptions {
                similarity: sim,
                json: true,
                fix: false,
                dry_run: true,
                exclude: excludes.into_iter().collect(),
                include_tests: inc_tests,
                include_folders: includes,
                verbose: false,
                with_cst: true,
                progress_bar: None,
            };
            let (_, findings) =
                crate::commands::run_clones(&path_bufs, &clone_options, &mut std::io::sink())
                    .map_err(|error| format!("Clone analysis failed: {error}"))?;
            result.clones = findings;
        }

        crate::analyzer::apply_global_ignores(&mut result, config.cytoscnpy.ignore.as_deref());

        serde_json::to_string(&result).map_err(|e| format!("Serialization error: {e}"))
    });

    drop(signal_guard);
    py.check_signals()?;

    if crate::CANCELLED.load(std::sync::atomic::Ordering::Relaxed) {
        return Err(PyErr::new::<pyo3::exceptions::PyKeyboardInterrupt, _>(
            "Operation cancelled by user",
        ));
    }

    json_result.map_err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>)
}

/// In-process scan of a single Python source code string returning raw JSON results.
#[pyfunction]
#[pyo3(signature = (
    code,
    filename = "<stdin>".to_owned(),
    confidence = 60,
    secrets = true,
    danger = true,
    quality = true
))]
fn scan_code_json(
    py: Python,
    code: String,
    filename: String,
    confidence: u8,
    secrets: bool,
    danger: bool,
    quality: bool,
) -> PyResult<String> {
    crate::CANCELLED.store(false, std::sync::atomic::Ordering::SeqCst);

    validate_confidence(confidence)?;
    let signal_guard = SignalGuard::new();

    let json_result = py.detach(move || {
        let analyzer = crate::analyzer::CytoScnPy::default()
            .with_confidence(confidence)
            .with_secrets(secrets)
            .with_danger(danger)
            .with_quality(quality);

        let mut result = analyzer.analyze_code(&code, &PathBuf::from(&filename));
        result.analysis_summary.secrets_count = result.secrets.len();
        result.analysis_summary.danger_count = result.danger.len();
        result.analysis_summary.quality_count = result.quality.len();
        result.analysis_summary.parse_errors_count = result.parse_errors.len();
        result.analysis_summary.taint_count = result.taint_findings.len();

        serde_json::to_string(&result).map_err(|e| format!("Serialization error: {e}"))
    });

    drop(signal_guard);
    py.check_signals()?;

    if crate::CANCELLED.load(std::sync::atomic::Ordering::Relaxed) {
        return Err(PyErr::new::<pyo3::exceptions::PyKeyboardInterrupt, _>(
            "Operation cancelled by user",
        ));
    }

    json_result.map_err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>)
}

/// Registers all Python functions with the module.
///
/// This function is called from `lib.rs` to populate the Python module
/// with all exposed functions.
pub(crate) fn register_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(run, m)?)?;
    m.add_function(wrap_pyfunction!(scan_json, m)?)?;
    m.add_function(wrap_pyfunction!(scan_code_json, m)?)?;
    Ok(())
}

fn validate_confidence(confidence: u8) -> PyResult<()> {
    if confidence > 100 {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "confidence must be between 0 and 100",
        ));
    }
    Ok(())
}
