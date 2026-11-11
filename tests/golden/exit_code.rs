use pyo3::prelude::*;
use steelix_runtime as rt;

// exit_code.py:1
#[rt::frame(m, "report", 1)]
pub fn report<'py>(m: &rt::Module<'py>, code: &Bound<'py, PyAny>) -> PyResult<()> {
    let py = m.py();
    rt::builtins::print(py, ("exiting with", code))?;
    rt::builtin(py, "exit")?.call1((code,))?;
    Ok(())
}

/// Executes the body of `exit_code.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::def!(m, report(code))?;
    report(m, &rt::int(py, 3))?;
    rt::builtins::print(py, ("not reached",))?;
    Ok(())
}
