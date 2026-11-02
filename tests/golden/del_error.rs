use pyo3::prelude::*;
use steelix_runtime as rt;

// del_error.py:1
pub fn drop<'py>(py: Python<'py>, flag: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let mut value: Option<Bound<'py, PyAny>> = None;
    if flag.is_truthy()? {
        value = Some(rt::int(py, 1));
    }
    rt::del_local(&mut value, "value")?;
    Ok(rt::str(py, "dropped"))
}

/// Executes the body of `del_error.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    rt::def!(globals, drop(flag))?;
    rt::builtins::print(py, (drop(py, &rt::bool(py, true))?,))?;
    rt::builtins::print(py, (drop(py, &rt::bool(py, false))?,))?;
    Ok(())
}
