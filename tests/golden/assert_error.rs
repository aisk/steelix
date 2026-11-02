use pyo3::exceptions::PyAssertionError;
use pyo3::prelude::*;
use steelix_runtime as rt;

// assert_error.py:1
pub fn positive<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "positive", 1, |frame| {
        frame.line(2);
        if !x.gt(0)? {
            return Err(frame.raise(PyAssertionError::new_err((rt::fstring(
                m.py(),
                ("expected a positive number, got ", rt::format(x, "")?),
            )?
            .unbind(),))));
        }
        Ok(x.clone())
    })
}

/// Executes the body of `assert_error.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        rt::def!(m, positive(x))?;
        frame.line(6);
        rt::builtins::print(py, (positive(m, &rt::int(py, 1))?,))?;
        frame.line(7);
        rt::builtins::print(py, (positive(m, &rt::int(py, -1))?,))?;
        Ok(())
    })
}
