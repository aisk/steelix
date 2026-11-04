use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// handler_release.py:19
pub fn gone<'py>(m: &rt::Module<'py>, r#ref: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "gone", 19, |_| {
        rt::builtins::print(py, ("weakref gone",))?;
        Ok(rt::none(py))
    })
}

// handler_release.py:23
pub fn raise_big<'py>(m: &rt::Module<'py>, tag: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "raise_big", 23, |frame| {
        Err(frame.raise(m.global("Big")?.call1((tag,))?))
    })
}

// handler_release.py:27
pub fn empty_handler<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "empty_handler", 27, |frame| {
        match (|| -> PyResult<()> { Err(frame.raise(m.global("Big")?.call1(("empty",))?)) })() {
            Ok(()) => unreachable!(),
            Err(err) if frame.matches(&err, 27, || m.global("Big"))? => {}
            Err(err) => return Err(err),
        }
        rt::builtins::print(py, ("after empty handler",))?;
        Ok(rt::none(py))
    })
}

// handler_release.py:35
pub fn builtin_class<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "builtin_class", 35, |frame| {
        let mut arg = Some(m.global("Arg")?.call1(("value-error",))?);
        let r#ref = m
            .global("weakref")?
            .attr("ref")?
            .call1((rt::local(&arg, "arg")?, m.global("gone")?))?;
        match (|| -> PyResult<()> {
            Err(frame.raise(PyValueError::new_err((rt::local(&arg, "arg")?
                .clone()
                .unbind(),))))
        })() {
            Ok(()) => unreachable!(),
            Err(err) if err.is_instance_of::<PyValueError>(py) => {}
            Err(err) => return Err(err),
        }
        arg = None;
        rt::builtins::print(py, ("after builtin handler", r#ref.call0()?.is_none()))?;
        Ok(rt::none(py))
    })
}

// handler_release.py:46
pub fn from_callee<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "from_callee", 46, |frame| {
        match raise_big(m, &rt::str(py, "callee")) {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 46, || m.global("Big"))? => {}
            Err(err) => return Err(err),
        }
        rt::builtins::print(py, ("after callee handler",))?;
        Ok(rt::none(py))
    })
}

// handler_release.py:54
pub fn handler_continue<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "handler_continue", 54, |frame| {
        for i in rt::builtin(py, "range")?.call1((2,))?.try_iter()? {
            let i = i?;
            match (|| -> PyResult<()> {
                Err(frame.raise(
                    m.global("Big")?
                        .call1((rt::fstring(py, ("continue ", rt::format(&i, "")?))?,))?,
                ))
            })() {
                Ok(()) => unreachable!(),
                Err(err) if frame.matches(&err, 54, || m.global("Big"))? => {
                    py.check_signals()?;
                    continue;
                }
                Err(err) => return Err(err),
            }
        }
        rt::builtins::print(py, ("after loop",))?;
        Ok(rt::none(py))
    })
}

// handler_release.py:63
pub fn handler_break<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "handler_break", 63, |frame| {
        loop {
            match (|| -> PyResult<()> { Err(frame.raise(m.global("Big")?.call1(("break",))?)) })() {
                Ok(()) => unreachable!(),
                Err(err) if frame.matches(&err, 63, || m.global("Big"))? => break,
                Err(err) => return Err(err),
            }
        }
        rt::builtins::print(py, ("after break",))?;
        Ok(rt::none(py))
    })
}

// handler_release.py:72
pub fn reraise_caught_outside<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "reraise_caught_outside", 72, |frame| {
        match (|| -> PyResult<()> {
            match (|| -> PyResult<()> { Err(frame.raise(m.global("Big")?.call1(("inner",))?)) })() {
                Ok(()) => unreachable!(),
                Err(err) if frame.matches(&err, 72, || m.global("Big"))? => {
                    let _caught = frame.catch(err);
                    Err(frame.reraise())
                }
                Err(err) => Err(err),
            }
        })() {
            Ok(()) => unreachable!(),
            Err(err) if frame.matches(&err, 72, || m.global("Big"))? => {}
            Err(err) => return Err(err),
        }
        rt::builtins::print(py, ("after nested",))?;
        Ok(rt::none(py))
    })
}

// handler_release.py:83
pub fn finally_then_handler<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "finally_then_handler", 83, |frame| {
        match (|| -> PyResult<()> {
            let result =
                (|| -> PyResult<()> { Err(frame.raise(m.global("Big")?.call1(("finally",))?)) })();
            let finally = frame.finally(result);
            rt::builtins::print(py, ("finally runs",))?;
            finally.resume()?;
            unreachable!();
        })() {
            Ok(()) => unreachable!(),
            Err(err) if frame.matches(&err, 83, || m.global("Big"))? => {}
            Err(err) => return Err(err),
        }
        rt::builtins::print(py, ("after finally",))?;
        Ok(rt::none(py))
    })
}

// handler_release.py:94
pub fn handler_as<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "handler_as", 94, |frame| {
        match (|| -> PyResult<()> { Err(frame.raise(m.global("Big")?.call1(("as",))?)) })() {
            Ok(()) => unreachable!(),
            Err(err) if frame.matches(&err, 94, || m.global("Big"))? => {
                let err = frame.catch(err);
                rt::builtins::print(py, ("handling", &err))?;
            }
            Err(err2) => return Err(err2),
        }
        rt::builtins::print(py, ("after as",))?;
        Ok(rt::none(py))
    })
}

// handler_release.py:102
pub fn frame_names<'py>(
    m: &rt::Module<'py>,
    err: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "frame_names", 102, |_| {
        let names = PyList::empty(py).into_any();
        let mut tb = err.attr("__traceback__")?;
        while !tb.is_none() {
            names
                .attr("append")?
                .call1((tb.attr("tb_frame")?.attr("f_code")?.attr("co_name")?,))?;
            tb = tb.attr("tb_next")?;
            py.check_signals()?;
        }
        Ok(names)
    })
}

// handler_release.py:111
pub fn address_reused<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "address_reused", 111, |frame| {
        // The exception from the callee usually gets the address of the one
        // raised here, which is gone by then; it still gets this frame in its
        // traceback.
        for i in rt::builtin(py, "range")?.call1((3,))?.try_iter()? {
            let i = i?;
            match (|| -> PyResult<()> {
                Err(frame.raise(
                    m.global("Big")?
                        .call1((rt::fstring(py, ("here ", rt::format(&i, "")?))?,))?,
                ))
            })() {
                Ok(()) => unreachable!(),
                Err(err2) if frame.matches(&err2, 111, || m.global("Big"))? => {}
                Err(err2) => return Err(err2),
            }
            match (|| raise_big(m, &rt::fstring(py, ("callee ", rt::format(&i, "")?))?))() {
                Ok(_) => {}
                Err(err) if frame.matches(&err, 111, || m.global("Big"))? => {
                    let err = frame.catch(err);
                    rt::builtins::print(py, ("traceback", frame_names(m, &err)?))?;
                }
                Err(err2) => return Err(err2),
            }
            py.check_signals()?;
        }
        Ok(rt::none(py))
    })
}

// handler_release.py:126
pub fn leaves<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "leaves", 126, |frame| {
        match (|| -> PyResult<()> { Err(frame.raise(m.global("Big")?.call1(("leaves",))?)) })() {
            Ok(()) => unreachable!(),
            Err(err) if frame.matches(&err, 126, || m.global("Big"))? => {}
            Err(err) => return Err(err),
        }
        raise_big(m, &rt::str(py, "uncaught here"))?;
        Ok(rt::none(py))
    })
}

pub mod big {
    use super::*;

    // handler_release.py:9
    /// The body of `class Big`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Big", 9, |_| {
            rt::def!(ns, __del__(slf as "self"))?;
            Ok(())
        })
    }

    // handler_release.py:10
    pub fn __del__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Big.__del__", 10, |_| {
            rt::builtins::print(py, ("del", slf.attr("args")?.get_item(0)?))?;
            Ok(rt::none(py))
        })
    }
}

pub mod arg {
    use super::*;

    // handler_release.py:14
    /// The body of `class Arg`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Arg", 14, |_| {
            rt::def!(ns, __init__(slf as "self", tag))?;
            Ok(())
        })
    }

    // handler_release.py:15
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        tag: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Arg.__init__", 15, |_| {
            slf.set_attr("tag", tag)?;
            Ok(rt::none(m.py()))
        })
    }
}

/// Executes the body of `handler_release.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "An exception caught by a handler that does not use it is released when the\n\
                handler ends, like in CPython, not when the function returns: its `__del__`\n\
                runs, and weakrefs to its arguments die, before the next statement (design\n\
                §9). Raised here, raised by a callee, with builtin and user classes.",
        )?;
        m.set_global("weakref", rt::import(m, "weakref")?)?;
        m.set_global(
            "Big",
            rt::class!(m, "Big"(rt::builtin(py, "Exception")?), big::body, line = 9)?,
        )?;
        m.set_global(
            "Arg",
            rt::class!(m, "Arg"(), arg::body, line = 14, attributes = ["tag"])?,
        )?;
        rt::def!(m, gone(r#ref))?;
        rt::def!(m, raise_big(tag))?;
        rt::def!(m, empty_handler())?;
        rt::def!(m, builtin_class())?;
        rt::def!(m, from_callee())?;
        rt::def!(m, handler_continue())?;
        rt::def!(m, handler_break())?;
        rt::def!(m, reraise_caught_outside())?;
        rt::def!(m, finally_then_handler())?;
        rt::def!(m, handler_as())?;
        rt::def!(m, frame_names(err))?;
        rt::def!(m, address_reused())?;
        rt::def!(m, leaves())?;
        for item in PyTuple::new(
            py,
            [
                &m.global("empty_handler")?,
                &m.global("builtin_class")?,
                &m.global("from_callee")?,
                &m.global("handler_continue")?,
                &m.global("handler_break")?,
                &m.global("reraise_caught_outside")?,
                &m.global("finally_then_handler")?,
                &m.global("handler_as")?,
                &m.global("address_reused")?,
            ],
        )?
        .try_iter()?
        {
            m.set_global("case", item?)?;
            m.global("case")?.call0()?;
            rt::builtins::print(py, ("returned from", m.global("case")?.attr("__name__")?))?;
            py.check_signals()?;
        }
        match leaves(m) {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 1, || m.global("Big"))? => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                rt::builtins::print(py, ("caught", m.global("err")?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        rt::builtins::print(py, ("end",))?;
        Ok(())
    })
}
