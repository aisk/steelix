use pyo3::prelude::*;
use steelix_runtime as rt;

// uncaught.py:1
pub fn lookup<'py>(py: Python<'py>, found: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let mut result: Option<Bound<'py, PyAny>> = None;
    if found.is_truthy()? {
        result = Some(rt::int(py, 42));
    }
    Ok(rt::local(&result, "result")?.clone())
}

/// Executes the body of `uncaught.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    rt::def!(globals, lookup(found))?;
    rt::builtins::print(py, (lookup(py, &rt::bool(py, true))?,))?;
    rt::builtins::print(py, (lookup(py, &rt::bool(py, false))?,))?;
    rt::builtins::print(py, ("not reached",))?;
    Ok(())
}
