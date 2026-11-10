use pyo3::prelude::*;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// plugins/count.py:4
#[rt::frame(m, "run", 4)]
pub fn run<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::int(m.py(), m.global("greet")?.method0("run")?.len()?))
}

/// Executes the body of `plugins/count.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let package = rt::import_names(m, ".", &["greet"])?;
    m.set_global("greet", rt::import_from(&package, "greet")?)?;
    rt::def!(m, run())?;
    Ok(())
}
