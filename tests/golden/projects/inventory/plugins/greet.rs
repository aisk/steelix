use pyo3::prelude::*;
use steelix_runtime as rt;

// plugins/greet.py:1
pub fn run<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::str(m.py(), "hello"))
}

/// Executes the body of `plugins/greet.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    rt::frame!(m, "<module>", 1, |_| {
        rt::def!(m, run())?;
        Ok(())
    })
}
