//! Python bindings for the CytoScnPy analyzer.
//!
//! This module provides PyO3 bindings that expose Rust functionality to Python.
//! It creates the `cytoscnpy` Python module with `run`, `scan_json`, and `scan_code_json`.

use pyo3::types::PyModuleMethods;
use pyo3::{pyfunction, types::PyModule, wrap_pyfunction, Bound, PyErr, PyResult, Python};
use std::path::PathBuf;

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

    // Release the GIL while running the Rust code
    let result = py.detach(|| {
        crate::entry_point::run_with_args(args)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    });

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

    let config = crate::config::Config::try_load_from_path(&root).unwrap_or_default();

    let conf = confidence.or(config.cytoscnpy.confidence).unwrap_or(60);
    let sec = secrets.or(config.cytoscnpy.secrets).unwrap_or(false);
    let dan = danger.or(config.cytoscnpy.danger).unwrap_or(false);
    let qua = quality.or(config.cytoscnpy.quality).unwrap_or(false);
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

    let json_result = py.detach(move || {
        let mut analyzer = crate::analyzer::CytoScnPy::new(
            conf,
            sec,
            dan,
            qua,
            inc_tests,
            excludes,
            includes,
            inc_ipynb,
            false,
            config.clone(),
        )
        .with_root(root);

        let mut result = analyzer.analyze_paths(&path_bufs);

        if run_clones {
            let clone_config = crate::clones::CloneConfig {
                min_similarity: sim,
                include_tests: inc_tests,
                ..Default::default()
            };
            if let Ok(detector) = crate::clones::CloneDetector::with_config(clone_config) {
                let clone_result = detector.detect_from_paths(&path_bufs);
                let clone_findings =
                    crate::commands::generate_clone_findings_with_thresholds(
                        &clone_result.pairs,
                        &[],
                        false,
                        90,
                        60,
                    );
                result.clones = clone_findings;
            }
        }

        crate::analyzer::apply_global_ignores(&mut result, config.cytoscnpy.ignore.as_deref());

        serde_json::to_string(&result).map_err(|e| format!("Serialization error: {e}"))
    });

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

    let json_result = py.detach(move || {
        let analyzer = crate::analyzer::CytoScnPy::default()
            .with_confidence(confidence)
            .with_secrets(secrets)
            .with_danger(danger)
            .with_quality(quality);

        let result = analyzer.analyze_code(&code, &PathBuf::from(&filename));

        serde_json::to_string(&result).map_err(|e| format!("Serialization error: {e}"))
    });

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
