//! Parts of `mixed_uncaught.py` kept as Python source and run by
//! `rt::exec_python!` in `exec_module` (steelix does not lower them yet):
//!
//! - line 8, `def python_kept`: list comprehension

use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;

// Tracebacks through code kept as Python show its file and line numbers.
// mixed_uncaught.py:4
pub fn native<'py>(m: &rt::Module<'py>, values: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    m.global("python_kept")?.call1((values,))
}

/// Executes the body of `mixed_uncaught.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::def!(m, native(values))?;
    // mixed_uncaught.py:8, kept as Python: list comprehension at 9:12
    rt::exec_python!(
        m,
        line = 8,
        r#"
def python_kept(values):
    return [10 // v for v in values]
"#
    )?;
    rt::builtins::print(py, (native(m, PyList::new(py, [1, 2])?.as_any())?,))?;
    rt::builtins::print(py, (native(m, PyList::new(py, [1, 0])?.as_any())?,))?;
    Ok(())
}
