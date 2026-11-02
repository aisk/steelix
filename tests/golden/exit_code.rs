use pyo3::prelude::*;
use steelix_runtime as rt;

// exit_code.py:1
pub fn report<'py>(m: &rt::Module<'py>, code: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "report", 1, |frame| {
        frame.line(2);
        rt::builtins::print(py, ("exiting with", code))?;
        frame.line(3);
        rt::builtin(py, "exit")?.call1((code,))?;
        Ok(rt::none(py))
    })
}

/// Executes the body of `exit_code.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        rt::def!(m, report(code))?;
        frame.line(6);
        report(m, &rt::int(py, 3))?;
        frame.line(7);
        rt::builtins::print(py, ("not reached",))?;
        Ok(())
    })
}
