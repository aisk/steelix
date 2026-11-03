use pyo3::prelude::*;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// plugins/count.py:4
pub fn run<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "run", 4, |_| {
        Ok(rt::int(m.py(), m.global("greet")?.method0("run")?.len()?))
    })
}

/// Executes the body of `plugins/count.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    rt::frame!(m, "<module>", 1, |_| {
        let package = rt::import_names(m, ".", &["greet"])?;
        m.set_global("greet", rt::import_from(&package, "greet")?)?;
        rt::def!(m, run())?;
        Ok(())
    })
}
