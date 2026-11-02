use pyo3::prelude::*;
use steelix_runtime as rt;

// uncaught.py:1
pub fn lookup<'py>(m: &rt::Module<'py>, found: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "lookup", 1, |frame| {
        let mut result: Option<Bound<'py, PyAny>> = None;
        frame.line(2);
        if found.is_truthy()? {
            result = Some(rt::int(m.py(), 42));
        }
        frame.line(4);
        Ok(rt::local(&result, "result")?.clone())
    })
}

/// Executes the body of `uncaught.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        rt::def!(m, lookup(found))?;
        frame.line(7);
        rt::builtins::print(py, (lookup(m, &rt::bool(py, true))?,))?;
        frame.line(8);
        rt::builtins::print(py, (lookup(m, &rt::bool(py, false))?,))?;
        frame.line(9);
        rt::builtins::print(py, ("not reached",))?;
        Ok(())
    })
}
