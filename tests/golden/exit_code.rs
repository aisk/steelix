use pyo3::prelude::*;
use steelix_runtime as rt;

// exit_code.py:1
pub fn report<'py>(py: Python<'py>, code: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::builtins::print(py, ("exiting with", code))?;
    rt::builtin(py, "exit")?.call1((code,))?;
    Ok(rt::none(py))
}

/// Executes the body of `exit_code.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    rt::def!(globals, report(code))?;
    report(py, &rt::int(py, 3))?;
    rt::builtins::print(py, ("not reached",))?;
    Ok(())
}
