use pyo3::prelude::*;
use steelix_runtime as rt;

// uncaught.py:1
pub fn lookup<'py>(m: &rt::Module<'py>, found: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "lookup", 1, |_| {
        let mut result: Option<Bound<'py, PyAny>> = None;
        if found.is_truthy()? {
            result = Some(rt::int(m.py(), 42));
        }
        Ok(rt::local(&result, "result")?.clone())
    })
}

/// Executes the body of `uncaught.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        rt::def!(m, lookup(found))?;
        rt::builtins::print(py, (lookup(m, &rt::bool(py, true))?,))?;
        rt::builtins::print(py, (lookup(m, &rt::bool(py, false))?,))?;
        rt::builtins::print(py, ("not reached",))?;
        Ok(())
    })
}
