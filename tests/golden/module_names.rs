use pyo3::prelude::*;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// module_names.py:4
pub fn m_<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "m", 4, |_| { x.add(1) })
}

// module_names.py:8
pub fn py_<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::str(m.py(), "py"))
}

// module_names.py:12
pub fn globals<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "globals", 12, |_| { a.mul(b) })
}

// module_names.py:16
pub fn exec_module_<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "exec_module", 16, |_| {
        m.global("m")?
            .call1((1,))?
            .add(m.global("globals")?.call1((3,))?)
    })
}

// module_names.py:20
pub fn module_<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "module", 20, |_| { x.mul(2) })
}

// module_names.py:24
pub fn match_length<'py>(
    module: &rt::Module<'py>,
    text: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(module, "match_length", 24, |_| {
        // A local named like the context parameter of generated functions.
        let m = module.global("len")?.call1((text,))?;
        module.global("module")?.call1((&m,))
    })
}

/// Executes the body of `module_names.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global(
            "__doc__",
            "Module level functions named like the locals of the generated module code.",
        )?;
        rt::def!(m, m_ as "m"(x))?;
        rt::def!(m, py_ as "py"())?;
        rt::def!(m, globals(a, b = 2))?;
        rt::def!(m, exec_module_ as "exec_module"())?;
        rt::def!(m, module_ as "module"(x))?;
        rt::def!(m, match_length(text))?;
        m.global("print")?.call1((
            m.global("m")?.call1((1,))?,
            m.global("py")?.call0()?,
            m.global("globals")?.call1((4,))?,
            m.global("exec_module")?.call0()?,
            m.global("match_length")?.call1(("abc",))?,
        ))?;
        m.set_global("f", m.global("m")?)?;
        m.global("print")?.call1((
            m.global("f")?.call1((10,))?,
            m.global("m")?.attr("__name__")?,
            m.global("py")?.attr("__name__")?,
            m.global("exec_module")?.attr("__name__")?,
            m.global("module")?.attr("__name__")?,
        ))?;
        Ok(())
    })
}
