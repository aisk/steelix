use pyo3::prelude::*;
use steelix_runtime as rt;

// plugins/static.py:1
pub fn run<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::str(m.py(), "found statically"))
}

/// Executes the body of `plugins/static.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    rt::def!(m, run())?;
    Ok(())
}
