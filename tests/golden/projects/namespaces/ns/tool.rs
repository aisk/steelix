use pyo3::prelude::*;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// ns/tool.py:5
pub fn run<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "run", 5, |_| {
        m.global("helper")?
            .method1("double", (n,))?
            .add(m.global("LEAF")?.len()?)
    })
}

// ns/tool.py:9
pub fn crash<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "crash", 9, |_| {
        m.global("helper")?
            .method1("fail", ("from the namespace package",))?;
        Ok(rt::none(m.py()))
    })
}

/// Executes the body of `ns/tool.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    rt::frame!(m, "<module>", 1, |_| {
        let package = rt::import_names(m, ".", &["helper"])?;
        m.set_global("helper", rt::import_from(&package, "helper")?)?;
        let leaf = rt::import_names(m, ".deep.leaf", &["LEAF"])?;
        m.set_global("LEAF", rt::import_from(&leaf, "LEAF")?)?;
        rt::def!(m, run(n))?;
        rt::def!(m, crash())?;
        Ok(())
    })
}
