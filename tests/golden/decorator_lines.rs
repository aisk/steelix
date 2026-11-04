use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// decorator_lines.py:7
pub fn boom<'py>(m: &rt::Module<'py>, r#fn: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "boom", 7, |frame| {
        Err(
            frame.raise(PyValueError::new_err((rt::str(m.py(), "applying to ")
                .add(r#fn.attr("__qualname__")?)?
                .unbind(),))),
        )
    })
}

// decorator_lines.py:11
pub fn fails<'py>(m: &rt::Module<'py>, label: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "fails", 11, |frame| {
        Err(
            frame.raise(PyValueError::new_err((rt::str(m.py(), "evaluating ")
                .add(label)?
                .unbind(),))),
        )
    })
}

// decorator_lines.py:15
pub fn ok<'py>(m: &rt::Module<'py>, r#fn: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    Ok(r#fn.clone())
}

// decorator_lines.py:19
pub fn show<'py>(m: &rt::Module<'py>, exc: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "show", 19, |_| {
        rt::builtins::print(py, (exc.get_type().attr("__name__")?, exc))?;
        for frame_ in m
            .global("traceback")?
            .attr("extract_tb")?
            .call1((exc.attr("__traceback__")?,))?
            .try_iter()?
        {
            let frame_ = frame_?;
            rt::builtins::print(
                py,
                (
                    " ",
                    frame_.attr("name")?,
                    frame_.attr("lineno")?,
                    frame_.attr("line")?,
                ),
            )?;
            py.check_signals()?;
        }
        Ok(rt::none(py))
    })
}

// decorator_lines.py:101
pub fn nested<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "nested", 101, |_| {
        // decorator_lines.py:104
        fn inner<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
            Ok(rt::none(m.py()))
        }
        let inner = rt::function!(
            m,
            @ok
            @(fails(m, &rt::str(m.py(), "in nested"))?)
            inner(),
            qualname = "nested.<locals>.inner"
        )?;
        Ok(inner)
    })
}

pub mod holder {
    use super::*;

    // decorator_lines.py:67
    /// The body of `class Holder`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Holder", 67, |_| {
            ns.set("attr", m.py().None())?;
            Ok(())
        })
    }
}

pub mod in_body {
    use super::*;

    // decorator_lines.py:77
    /// The body of `class InBody`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "InBody", 77, |_| {
            rt::def!(ns, @ok @holder.attr.boom method(slf as "self"))?;
            Ok(())
        })
    }

    // decorator_lines.py:80
    pub fn method<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::none(m.py()))
    }
}

pub mod single {
    use super::*;

    // decorator_lines.py:88
    /// The body of `class Single`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Single", 88, |_| {
            rt::def!(ns, @staticmethod fine())?;
            rt::def!(ns, @boom method(slf as "self"))?;
            Ok(())
        })
    }

    // decorator_lines.py:90
    pub fn fine<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::none(m.py()))
    }

    // decorator_lines.py:94
    pub fn method<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::none(m.py()))
    }
}

/// Executes the body of `decorator_lines.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        // Traceback lines of decorators that raise when evaluated or applied, and of
        // defaults that raise in a decorated def: CPython reports the line of the
        // decorator (or of the default), not that of `def`.
        m.set_global("traceback", rt::import(m, "traceback")?)?;
        rt::def!(m, boom(r#fn))?;
        rt::def!(m, fails(label))?;
        rt::def!(m, ok(r#fn))?;
        rt::def!(m, show(exc))?;
        match (|| -> PyResult<()> {
            // decorator_lines.py:30
            fn evaluated<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
                Ok(rt::none(m.py()))
            }
            rt::def!(m, @ok @(fails(m, &rt::str(py, "second"))?) @ok evaluated())?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyValueError>(py) => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                show(m, &m.global("e")?)?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        match (|| -> PyResult<()> {
            // decorator_lines.py:40
            fn defaulted<'py>(
                m: &rt::Module<'py>,
                x: &Bound<'py, PyAny>,
            ) -> PyResult<Bound<'py, PyAny>> {
                Ok(rt::none(m.py()))
            }
            rt::def!(m, @ok @ok defaulted(x = fails(m, &rt::str(py, "default"))?))?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyValueError>(py) => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                show(m, &m.global("e")?)?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        match (|| -> PyResult<()> {
            // decorator_lines.py:51
            fn applied<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
                Ok(rt::none(m.py()))
            }
            rt::def!(m, @ok @boom @ok applied())?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyValueError>(py) => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                show(m, &m.global("e")?)?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        match (|| -> PyResult<()> {
            // decorator_lines.py:60
            fn single<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
                Ok(rt::none(m.py()))
            }
            rt::def!(m, @boom single())?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyValueError>(py) => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                show(m, &m.global("e")?)?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        m.set_global(
            "Holder",
            rt::class!(m, "Holder"(), holder::body, line = 67)?,
        )?;
        m.set_global("holder", m.global("Holder")?.call0()?)?;
        let value = m.global("Holder")?.call0()?;
        m.global("holder")?.set_attr("attr", value)?;
        let boom2 = m.global("boom")?;
        m.global("holder")?.attr("attr")?.set_attr("boom", boom2)?;
        match (|| -> PyResult<()> {
            m.set_global(
                "InBody",
                rt::class!(m, "InBody"(), in_body::body, line = 77)?,
            )?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyValueError>(py) => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                show(m, &m.global("e")?)?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        match (|| -> PyResult<()> {
            m.set_global(
                "Single",
                rt::class!(m, "Single"(), single::body, line = 88)?,
            )?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyValueError>(py) => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                show(m, &m.global("e")?)?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        rt::def!(m, nested())?;
        match nested(m) {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyValueError>(py) => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                show(m, &m.global("e")?)?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        Ok(())
    })
}
