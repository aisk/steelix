use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use steelix_runtime as rt;

// ns/helper.py:1
#[rt::frame(m, "double", 1)]
pub fn double<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    n.mul(2)
}

// ns/helper.py:5
#[rt::frame(m, "fail", 5, frame)]
pub fn fail<'py>(m: &rt::Module<'py>, message: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    Err(frame.raise(PyRuntimeError::new_err((message.clone().unbind(),))))
}

/// Executes the body of `ns/helper.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    rt::def!(m, double(n))?;
    rt::def!(m, fail(message))?;
    Ok(())
}
