use pyo3::prelude::*;
use steelix_runtime as rt;

// exit_code.py:1
pub fn report<'py>(m: &rt::Module<'py>, code: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "report", 1, |_| {
        rt::builtins::print(py, ("exiting with", code))?;
        rt::builtin(py, "exit")?.call1((code,))?;
        Ok(rt::none(py))
    })
}

/// Executes the body of `exit_code.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        rt::def!(m, report(code))?;
        report(m, &rt::int(py, 3))?;
        rt::builtins::print(py, ("not reached",))?;
        Ok(())
    })
}
