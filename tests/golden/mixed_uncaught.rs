//! Parts of `mixed_uncaught.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 8, `def python_kept`: generator expression with several for clauses

use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;

// Tracebacks through code kept as Python show its file and line numbers.
// mixed_uncaught.py:4
pub fn native<'py>(m: &rt::Module<'py>, values: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "native", 4, |_| {
        m.global("python_kept")?.call1((values,))
    })
}

/// Executes the body of `mixed_uncaught.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        rt::def!(m, native(values))?;
        // mixed_uncaught.py:8, kept as Python: generator expression with several for clauses at 9:16
        rt::exec_python!(
            m,
            line = 8,
            r#"
def python_kept(values):
    return list(v for v in [*values, 10 // values[-1]] for _ in "1")
"#
        )?;
        rt::builtins::print(py, (native(m, PyList::new(py, [1, 2])?.as_any())?,))?;
        rt::builtins::print(py, (native(m, PyList::new(py, [1, 0])?.as_any())?,))?;
        Ok(())
    })
}
