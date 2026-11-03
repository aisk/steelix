use pyo3::prelude::*;
use steelix_runtime as rt;

// module_setattr.py:7
pub fn plain<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "plain", 7, |frame| {
        frame.line(8);
        x.mul(2)
    })
}

// module_setattr.py:11
pub fn other<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "other", 11, |frame| {
        frame.line(12);
        x.add(1)
    })
}

/// Executes the body of `module_setattr.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    rt::frame!(m, "<module>", 1, |frame| {
        // Rebinding a function through the module object or `exec` is seen by later
        // calls: such modules call their functions through globals (design §7).
        frame.line(3);
        m.set_global("sys", rt::import(m, "sys")?)?;
        frame.line(4);
        m.set_global("types", rt::import(m, "types")?)?;
        rt::def!(m, plain(x))?;
        rt::def!(m, other(x))?;
        frame.line(15);
        m.global("print")?.call1((
            m.global("plain")?.call1((21,))?,
            m.global("other")?.call1((1,))?,
        ))?;
        // module_setattr.py:16
        fn lambda<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "<lambda>", 16, |_| x.neg())
        }
        frame.line(16);
        m.global("setattr")?.call1((
            m.global("sys")?
                .getattr("modules")?
                .get_item(m.global("__name__")?)?,
            "plain",
            rt::function!(m, lambda(x), qualname = "<lambda>")?,
        ))?;
        frame.line(17);
        m.global("print")?
            .call1((m.global("plain")?.call1((21,))?,))?;
        frame.line(18);
        m.set_global(
            "me",
            m.global("sys")?
                .getattr("modules")?
                .get_item(m.global("__name__")?)?,
        )?;
        frame.line(19);
        let plain2 = m.global("plain")?;
        m.global("me")?.setattr("other", plain2)?;
        frame.line(20);
        m.global("print")?
            .call1((m.global("other")?.call1((1,))?,))?;
        frame.line(21);
        m.global("exec")?.call1((
            "def plain(x):\n    return x * 10",
            m.globals().clone().into_any(),
        ))?;
        frame.line(22);
        m.global("print")?.call1((
            m.global("plain")?.call1((21,))?,
            m.global("other")?.call1((1,))?,
        ))?;
        // `vars(obj)` reads another namespace and does not count.
        frame.line(24);
        m.global("print")?
            .call1((m.global("vars")?.call1((rt::call!(
                m.global("types")?.getattr("SimpleNamespace")?,
                a = 1
            )?,))?,))?;
        Ok(())
    })
}
