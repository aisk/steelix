use pyo3::prelude::*;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// module_setattr.py:7
#[rt::frame(m, "plain", 7)]
pub fn plain<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    x.mul(2)
}

// module_setattr.py:11
#[rt::frame(m, "other", 11)]
pub fn other<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    x.add(1)
}

/// Executes the body of `module_setattr.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    // Rebinding a function through the module object or `exec` is seen by later
    // calls: such modules call their functions through globals (design §7).
    m.set_global("sys", rt::import(m, "sys")?)?;
    m.set_global("types", rt::import(m, "types")?)?;
    rt::def!(m, plain(x))?;
    rt::def!(m, other(x))?;
    m.global("print")?.call1((
        m.global("plain")?.call1((21,))?,
        m.global("other")?.call1((1,))?,
    ))?;
    // module_setattr.py:16
    #[rt::frame(m, "<lambda>", 16)]
    fn lambda<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        x.neg()
    }
    m.global("setattr")?.call1((
        m.global("sys")?
            .attr("modules")?
            .get_item(m.global("__name__")?)?,
        "plain",
        rt::function!(m, lambda(x), qualname = "<lambda>")?,
    ))?;
    m.global("print")?
        .call1((m.global("plain")?.call1((21,))?,))?;
    m.set_global(
        "me",
        m.global("sys")?
            .attr("modules")?
            .get_item(m.global("__name__")?)?,
    )?;
    let plain2 = m.global("plain")?;
    m.global("me")?.set_attr("other", plain2)?;
    m.global("print")?
        .call1((m.global("other")?.call1((1,))?,))?;
    m.global("exec")?
        .call1(("def plain(x):\n    return x * 10", m.globals()))?;
    m.global("print")?.call1((
        m.global("plain")?.call1((21,))?,
        m.global("other")?.call1((1,))?,
    ))?;
    // `vars(obj)` reads another namespace and does not count.
    m.global("print")?
        .call1((m.global("vars")?.call1((rt::call!(
            m.global("types")?.attr("SimpleNamespace")?,
            a = 1
        )?,))?,))?;
    Ok(())
}
