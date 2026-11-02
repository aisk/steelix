use pyo3::exceptions::PyAssertionError;
use pyo3::prelude::*;
use steelix_runtime as rt;

// assert_error.py:1
pub fn positive<'py>(py: Python<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    if !x.gt(0)? {
        return Err(PyAssertionError::new_err((rt::fstring(
            py,
            ("expected a positive number, got ", rt::format(x, "")?),
        )?
        .unbind(),)));
    }
    Ok(x.clone())
}

/// Executes the body of `assert_error.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    rt::def!(globals, positive(x))?;
    rt::builtins::print(py, (positive(py, &rt::int(py, 1))?,))?;
    rt::builtins::print(py, (positive(py, &rt::int(py, -1))?,))?;
    Ok(())
}
