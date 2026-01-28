//! Python bindings for the CytoScnPy analyzer.
//!
//! This module provides PyO3 bindings that expose Rust functionality to Python.
//! It creates the `cytoscnpy` Python module with a `run` function.

use pyo3::types::PyModuleMethods;
use pyo3::{pyfunction, types::PyModule, wrap_pyfunction, Bound, PyErr, PyResult, Python};

/// Python-callable wrapper for the analyzer.
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

    // Register Ctrl+C handler
    // We ignore the error if a handler is already set (e.g. from a previous run)
    let _ = ctrlc::set_handler(|| {
        crate::CANCELLED.store(true, std::sync::atomic::Ordering::SeqCst);
    });

    // Release the GIL while running the Rust code
    let result = py.detach(|| {
        crate::entry_point::run_with_args(args)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
    });

    // If cancelled, ensure we raise KeyboardInterrupt for Python to handle
    if crate::CANCELLED.load(std::sync::atomic::Ordering::Relaxed) {
        return Err(PyErr::new::<pyo3::exceptions::PyKeyboardInterrupt, _>(
            "Operation cancelled by user",
        ));
    }

    result
}

/// Registers all Python functions with the module.
///
/// This function is called from `lib.rs` to populate the Python module
/// with all exposed functions.
pub(crate) fn register_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(run, m)?)?;
    Ok(())
}
