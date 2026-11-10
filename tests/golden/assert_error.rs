use pyo3::exceptions::PyAssertionError;
use pyo3::prelude::*;
use steelix_runtime as rt;

// assert_error.py:1
#[rt::frame(m, "positive", 1, frame)]
pub fn positive<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    if !x.gt(0)? {
        return Err(frame.raise(PyAssertionError::new_err((rt::fstring(
            m.py(),
            ("expected a positive number, got ", rt::format(x, "")?),
        )?
        .unbind(),))));
    }
    Ok(x.clone())
}

/// Executes the body of `assert_error.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::def!(m, positive(x))?;
    rt::builtins::print(py, (positive(m, &rt::int(py, 1))?,))?;
    rt::builtins::print(py, (positive(m, &rt::int(py, -1))?,))?;
    Ok(())
}
