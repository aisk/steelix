use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// with_stmt.py:45
pub fn write_and_read<'py>(
    m: &rt::Module<'py>,
    path: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "write_and_read", 45, |frame| {
        frame.with(&m.global("open")?.call1((path, "w"))?, |f| {
            f.method1("write", ("hello\n",))?;
            Ok(())
        })?;
        if let rt::Flow::Return(value) = frame.with(&m.global("open")?.call1((path,))?, |f| {
            Ok(rt::Flow::Return(f.method0("read")?))
        })? {
            return Ok(value);
        }
        Ok(rt::none(m.py()))
    })
}

// with_stmt.py:52
pub fn target_kinds<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "target_kinds", 52, |frame| {
        frame.with(&m.global("Manager")?.call1(("abc",))?, |value| {
            let name: Bound<'py, PyAny>;
            let size: Bound<'py, PyAny>;
            [name, size] = rt::unpack(value)?;
            m.global("print")?.call1(("unpacked", &name, &size))?;
            Ok(())
        })?;
        let holder = PyDict::new(py).into_any();
        frame.with(&m.global("Manager")?.call1(("d",))?, |value| {
            holder.set_item("value", value)?;
            Ok(())
        })?;
        m.global("print")?.call1(("stored", &holder))?;
        frame.with(&m.global("Manager")?.call1(("e",))?, |_| {
            m.global("print")?.call1(("no target",))?;
            Ok(())
        })?;
        Ok(rt::none(py))
    })
}

// with_stmt.py:63
pub fn multiple<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "multiple", 63, |frame| {
        frame.with(&m.global("Manager")?.call1(("outer",))?, |a| {
            frame.with(&m.global("Manager")?.call1(("inner",))?, |b| {
                m.global("print")?.call1(("both", a, b))?;
                Ok(())
            })
        })?;
        Ok(rt::none(m.py()))
    })
}

// with_stmt.py:68
pub fn suppressing<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "suppressing", 68, |frame| {
        frame.with(
            &rt::call!(m.global("Manager")?, "quiet", suppress = true)?,
            |_| -> PyResult<()> {
                Err(frame.raise(m.global("ValueError")?.call1(("suppressed",))?))
            },
        )?;
        m.global("print")?.call1(("after suppressed",))?;
        Ok(rt::none(m.py()))
    })
}

// with_stmt.py:74
pub fn not_suppressing<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "not_suppressing", 74, |frame| {
        match (|| -> PyResult<()> {
            frame.with(
                &m.global("Manager")?.call1(("loud",))?,
                |_| -> PyResult<()> {
                    Err(frame.raise(m.global("ValueError")?.call1(("propagates",))?))
                },
            )?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if frame.matches(&err, 74, || m.global("ValueError"))? => {
                let e = frame.catch(err);
                m.global("print")?.call1(("caught", &e))?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(m.py()))
    })
}

// with_stmt.py:82
pub fn enter_fails<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "enter_fails", 82, |frame| {
        match (|| -> PyResult<()> {
            frame.with(&m.global("Manager")?.call1(("first",))?, |_| {
                frame.with(
                    &rt::call!(m.global("Manager")?, "second", fail_enter = true)?,
                    |_| {
                        m.global("print")?.call1(("not reached",))?;
                        Ok(())
                    },
                )
            })?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if frame.matches(&err, 82, || m.global("RuntimeError"))? => {
                let e = frame.catch(err);
                m.global("print")?.call1(("caught", &e))?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(m.py()))
    })
}

// with_stmt.py:90
pub fn exit_fails<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "exit_fails", 90, |frame| {
        match (|| -> PyResult<()> {
            frame.with(
                &rt::call!(m.global("Manager")?, "ok", fail_exit = true)?,
                |_| -> PyResult<()> {
                    Err(frame.raise(m.global("ValueError")?.call1(("original",))?))
                },
            )?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if frame.matches(&err, 90, || m.global("KeyError"))? => {
                let e = frame.catch(err);
                m.global("print")?.call1((
                    "caught",
                    m.global("repr")?.call1((&e,))?,
                    "context",
                    m.global("repr")?.call1((e.attr("__context__")?,))?,
                ))?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(m.py()))
    })
}

// with_stmt.py:98
pub fn jumps<'py>(m: &rt::Module<'py>, items: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "jumps", 98, |frame| {
        let mut found = rt::none(py);
        for item in rt::iter(items)? {
            let item = item?;
            match frame.with(&m.global("Manager")?.call1((&item,))?, |_| {
                if item.eq("skip")? {
                    return Ok(rt::Flow::Continue);
                }
                if item.eq("stop")? {
                    return Ok(rt::Flow::Break);
                }
                if item.eq("ret")? {
                    return Ok(rt::Flow::Return(rt::str(py, "returned ").add(&item)?));
                }
                found = item.clone();
                Ok(rt::Flow::Next)
            })? {
                rt::Flow::Return(value) => return Ok(value),
                rt::Flow::Break => break,
                rt::Flow::Continue => continue,
                rt::Flow::Next => {}
            }
        }
        Ok(found)
    })
}

// with_stmt.py:112
pub fn returns_value<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "returns_value", 112, |frame| {
        if let rt::Flow::Return(value) = frame
            .with(&m.global("Manager")?.call1(("value",))?, |v| {
                Ok(rt::Flow::Return(v.clone()))
            })?
        {
            return Ok(value);
        }
        Ok(rt::none(m.py()))
    })
}

// with_stmt.py:117
pub fn bad_managers<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "bad_managers", 117, |frame| {
        for manager in rt::iter(
            PyList::new(
                py,
                [
                    &m.global("NoExit")?.call0()?,
                    &m.global("Async")?.call0()?,
                    &rt::int(py, 42),
                ],
            )?
            .as_any(),
        )? {
            let manager = manager?;
            match (|| -> PyResult<()> {
                frame.with(&manager, |_| Ok(()))?;
                Ok(())
            })() {
                Ok(()) => {}
                Err(err) if frame.matches(&err, 117, || m.global("TypeError"))? => {
                    let e = frame.catch(err);
                    m.global("print")?.call1(("TypeError:", &e))?;
                }
                Err(err) => return Err(err),
            }
        }
        Ok(rt::none(py))
    })
}

// with_stmt.py:126
pub fn lookup_on_type<'py>(module: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(module, "lookup_on_type", 126, |frame| {
        let m = module.global("InstanceMethods")?.call0()?;
        // with_stmt.py:128
        fn lambda<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
            Ok(rt::none(m.py()))
        }
        m.set_attr(
            "__enter__",
            rt::function!(
                module,
                lambda(),
                qualname = "lookup_on_type.<locals>.<lambda>"
            )?,
        )?;
        // with_stmt.py:129
        fn lambda2<'py>(
            m: &rt::Module<'py>,
            a: &Bound<'py, PyTuple>,
        ) -> PyResult<Bound<'py, PyAny>> {
            Ok(rt::none(m.py()))
        }
        m.set_attr(
            "__exit__",
            rt::function!(
                module,
                lambda2(*a),
                qualname = "lookup_on_type.<locals>.<lambda>"
            )?,
        )?;
        match (|| -> PyResult<()> {
            frame.with(&m, |_| Ok(()))?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if frame.matches(&err, 126, || module.global("TypeError"))? => {
                let e = frame.catch(err);
                module.global("print")?.call1(("TypeError:", &e))?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(module.py()))
    })
}

// with_stmt.py:137
pub fn suppress_in_loop<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "suppress_in_loop", 137, |frame| {
        let mut count = rt::int(m.py(), 0);
        for i in rt::iter(&m.global("range")?.call1((3,))?)? {
            let i = i?;
            frame.with(
                &rt::call!(
                    m.global("Manager")?,
                    m.global("str")?.call1((&i,))?,
                    suppress = true
                )?,
                |_| {
                    if i.eq(1)? {
                        return Err(frame.raise(m.global("ValueError")?.call1((&i,))?));
                    }
                    count = rt::iadd(&count, 1)?;
                    Ok(())
                },
            )?;
        }
        Ok(count)
    })
}

pub mod manager {
    use super::*;

    // with_stmt.py:7
    /// The body of `class Manager`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Manager", 7, |_| {
            rt::def!(
                ns,
                __init__(slf as "self", name, suppress = false, fail_enter = false, fail_exit = false)
            )?;
            rt::def!(ns, __enter__(slf as "self"))?;
            rt::def!(ns, __exit__(slf as "self", exc_type, exc, tb))?;
            Ok(())
        })
    }

    // with_stmt.py:8
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
        suppress: &Bound<'py, PyAny>,
        fail_enter: &Bound<'py, PyAny>,
        fail_exit: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Manager.__init__", 8, |_| {
            slf.set_attr("name", name)?;
            slf.set_attr("suppress", suppress)?;
            slf.set_attr("fail_enter", fail_enter)?;
            slf.set_attr("fail_exit", fail_exit)?;
            Ok(rt::none(m.py()))
        })
    }

    // with_stmt.py:14
    pub fn __enter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Manager.__enter__", 14, |frame| {
            m.global("print")?.call1(("enter", slf.attr("name")?))?;
            if slf.attr("fail_enter")?.is_truthy()? {
                return Err(frame.raise(
                    m.global("RuntimeError")?
                        .call1((rt::str(py, "enter failed: ").add(slf.attr("name")?)?,))?,
                ));
            }
            Ok(PyTuple::new(
                py,
                [
                    &slf.attr("name")?,
                    &m.global("len")?.call1((slf.attr("name")?,))?,
                ],
            )?
            .into_any())
        })
    }

    // with_stmt.py:20
    pub fn __exit__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        exc_type: &Bound<'py, PyAny>,
        exc: &Bound<'py, PyAny>,
        tb: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Manager.__exit__", 20, |frame| {
            m.global("print")?.call1((
                "exit",
                slf.attr("name")?,
                if exc_type.is_truthy()? {
                    exc_type.attr("__name__")?
                } else {
                    rt::none(py)
                },
                m.global("repr")?.call1((exc,))?,
                !tb.is_none(),
                "handled:",
                m.global("repr")?
                    .call1((m.global("sys")?.method0("exception")?,))?,
            ))?;
            if slf.attr("fail_exit")?.is_truthy()? {
                return Err(frame.raise(
                    m.global("KeyError")?
                        .call1((rt::str(py, "exit failed: ").add(slf.attr("name")?)?,))?,
                ));
            }
            slf.attr("suppress")
        })
    }
}

pub mod no_exit {
    use super::*;

    // with_stmt.py:28
    /// The body of `class NoExit`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "NoExit", 28, |_| {
            rt::def!(ns, __enter__(slf as "self"))?;
            Ok(())
        })
    }

    // with_stmt.py:29
    pub fn __enter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(slf.clone())
    }
}

pub mod async_ {
    use super::*;

    // with_stmt.py:33
    /// The body of `class Async`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Async", 33, |_| {
            rt::def!(ns, async __aenter__(slf as "self"))?;
            rt::def!(ns, async __aexit__(slf as "self", *args))?;
            Ok(())
        })
    }

    // with_stmt.py:34
    pub async fn __aenter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Async.__aenter__", 34, async |_| { Ok(slf.clone()) })
    }

    // with_stmt.py:37
    pub async fn __aexit__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        args: &Bound<'py, PyTuple>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Async.__aexit__", 37, async |_| {
            Ok(rt::bool(m.py(), false))
        })
    }
}

pub mod instance_methods {
    use super::*;

    // with_stmt.py:41
    /// The body of `class InstanceMethods`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

/// Executes the body of `with_stmt.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        // `with`: the protocol, suppression, targets, and jumps through `with`.
        m.set_global("os", rt::import(m, "os")?)?;
        m.set_global("sys", rt::import(m, "sys")?)?;
        m.set_global("tempfile", rt::import(m, "tempfile")?)?;
        m.set_global(
            "Manager",
            rt::class!(
                m,
                "Manager"(),
                manager::body,
                line = 7,
                attributes = ["fail_enter", "fail_exit", "name", "suppress"]
            )?,
        )?;
        m.set_global(
            "NoExit",
            rt::class!(m, "NoExit"(), no_exit::body, line = 28)?,
        )?;
        m.set_global("Async", rt::class!(m, "Async"(), async_::body, line = 33)?)?;
        m.set_global(
            "InstanceMethods",
            rt::class!(m, "InstanceMethods"(), instance_methods::body, line = 41)?,
        )?;
        rt::def!(m, write_and_read(path))?;
        rt::def!(m, target_kinds())?;
        rt::def!(m, multiple())?;
        rt::def!(m, suppressing())?;
        rt::def!(m, not_suppressing())?;
        rt::def!(m, enter_fails())?;
        rt::def!(m, exit_fails())?;
        rt::def!(m, jumps(items))?;
        rt::def!(m, returns_value())?;
        rt::def!(m, bad_managers())?;
        rt::def!(m, lookup_on_type())?;
        rt::def!(m, suppress_in_loop())?;
        m.set_global(
            "path",
            m.global("os")?
                .attr("path")?
                .attr("join")?
                .call1((m.global("tempfile")?.method0("mkdtemp")?, "data.txt"))?,
        )?;
        m.global("print")?
            .call1((m.global("write_and_read")?.call1((m.global("path")?,))?,))?;
        m.global("target_kinds")?.call0()?;
        m.global("multiple")?.call0()?;
        m.global("suppressing")?.call0()?;
        m.global("not_suppressing")?.call0()?;
        m.global("enter_fails")?.call0()?;
        m.global("exit_fails")?.call0()?;
        m.global("print")?.call1((m
            .global("jumps")?
            .call1((PyList::new(py, ["a", "skip", "b", "stop", "c"])?,))?,))?;
        m.global("print")?.call1((m
            .global("jumps")?
            .call1((PyList::new(py, ["a", "ret"])?,))?,))?;
        m.global("print")?
            .call1((m.global("returns_value")?.call0()?,))?;
        m.global("bad_managers")?.call0()?;
        m.global("lookup_on_type")?.call0()?;
        m.global("print")?
            .call1((m.global("suppress_in_loop")?.call0()?,))?;
        frame.with(&m.global("Manager")?.call1(("module",))?, |value| {
            m.set_global("pair", value)?;
            m.global("print")?
                .call1(("module level", m.global("pair")?))?;
            Ok(())
        })?;
        frame.with(
            &rt::call!(m.global("Manager")?, "module quiet", suppress = true)?,
            |_| -> PyResult<()> { Err(frame.raise(m.global("ValueError")?.call1(("module",))?)) },
        )?;
        m.global("print")?
            .call1((m.globals().contains("pair")?, m.global("pair")?))?;
        Ok(())
    })
}
