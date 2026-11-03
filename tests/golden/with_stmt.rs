//! Parts of `with_stmt.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 34, `def __aenter__`: async function
//! - line 37, `def __aexit__`: async function

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// with_stmt.py:45
pub fn write_and_read<'py>(
    m: &rt::Module<'py>,
    path: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "write_and_read", 45, |frame| {
        frame.line(46);
        frame.with(&m.global("open")?.call1((path, "w"))?, |f| {
            frame.line(47);
            f.call_method1("write", ("hello\n",))?;
            Ok(())
        })?;
        frame.line(48);
        if let rt::Flow::Return(value) = frame.with(&m.global("open")?.call1((path,))?, |f| {
            frame.line(49);
            Ok(rt::Flow::Return(f.call_method0("read")?))
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
        frame.line(53);
        frame.with(&m.global("Manager")?.call1(("abc",))?, |value| {
            let name: Bound<'py, PyAny>;
            let size: Bound<'py, PyAny>;
            [name, size] = rt::unpack(value)?;
            frame.line(54);
            m.global("print")?.call1(("unpacked", &name, &size))?;
            Ok(())
        })?;
        let holder = PyDict::new(py).into_any();
        frame.line(56);
        frame.with(&m.global("Manager")?.call1(("d",))?, |value| {
            holder.set_item("value", value)?;
            Ok(())
        })?;
        frame.line(58);
        m.global("print")?.call1(("stored", &holder))?;
        frame.line(59);
        frame.with(&m.global("Manager")?.call1(("e",))?, |_| {
            frame.line(60);
            m.global("print")?.call1(("no target",))?;
            Ok(())
        })?;
        Ok(rt::none(py))
    })
}

// with_stmt.py:63
pub fn multiple<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "multiple", 63, |frame| {
        frame.line(64);
        frame.with(&m.global("Manager")?.call1(("outer",))?, |a| {
            frame.with(&m.global("Manager")?.call1(("inner",))?, |b| {
                frame.line(65);
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
        frame.line(69);
        frame.with(
            &rt::call!(m.global("Manager")?, "quiet", suppress = true)?,
            |_| -> PyResult<()> {
                frame.line(70);
                Err(frame.raise(m.global("ValueError")?.call1(("suppressed",))?))
            },
        )?;
        frame.line(71);
        m.global("print")?.call1(("after suppressed",))?;
        Ok(rt::none(m.py()))
    })
}

// with_stmt.py:74
pub fn not_suppressing<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "not_suppressing", 74, |frame| {
        match (|| -> PyResult<()> {
            frame.line(76);
            frame.with(
                &m.global("Manager")?.call1(("loud",))?,
                |_| -> PyResult<()> {
                    frame.line(77);
                    Err(frame.raise(m.global("ValueError")?.call1(("propagates",))?))
                },
            )?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if frame.matches(&err, 78, || m.global("ValueError"))? => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                frame.line(79);
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
            frame.line(84);
            frame.with(&m.global("Manager")?.call1(("first",))?, |_| {
                frame.with(
                    &rt::call!(m.global("Manager")?, "second", fail_enter = true)?,
                    |_| {
                        frame.line(85);
                        m.global("print")?.call1(("not reached",))?;
                        Ok(())
                    },
                )
            })?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if frame.matches(&err, 86, || m.global("RuntimeError"))? => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                frame.line(87);
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
            frame.line(92);
            frame.with(
                &rt::call!(m.global("Manager")?, "ok", fail_exit = true)?,
                |_| -> PyResult<()> {
                    frame.line(93);
                    Err(frame.raise(m.global("ValueError")?.call1(("original",))?))
                },
            )?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if frame.matches(&err, 94, || m.global("KeyError"))? => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                frame.line(95);
                m.global("print")?.call1((
                    "caught",
                    m.global("repr")?.call1((&e,))?,
                    "context",
                    m.global("repr")?.call1((e.getattr("__context__")?,))?,
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
        frame.line(100);
        for item in items.try_iter()? {
            let item = item?;
            frame.line(101);
            match frame.with(&m.global("Manager")?.call1((&item,))?, |_| {
                frame.line(102);
                if item.eq("skip")? {
                    return Ok(rt::Flow::Continue);
                }
                frame.line(104);
                if item.eq("stop")? {
                    return Ok(rt::Flow::Break);
                }
                frame.line(106);
                if item.eq("ret")? {
                    frame.line(107);
                    return Ok(rt::Flow::Return(rt::str(py, "returned ").add(&item)?));
                }
                found = item.clone();
                Ok(rt::Flow::Next)
            })? {
                rt::Flow::Return(value) => return Ok(value),
                rt::Flow::Break => break,
                rt::Flow::Continue => {
                    py.check_signals()?;
                    frame.line(100);
                    continue;
                }
                rt::Flow::Next => {}
            }
            py.check_signals()?;
            frame.line(100);
        }
        Ok(found)
    })
}

// with_stmt.py:112
pub fn returns_value<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "returns_value", 112, |frame| {
        frame.line(113);
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
        frame.line(118);
        for manager in PyList::new(
            py,
            [
                &m.global("NoExit")?.call0()?,
                &m.global("Async")?.call0()?,
                &rt::int(py, 42),
            ],
        )?
        .try_iter()?
        {
            let manager = manager?;
            match (|| -> PyResult<()> {
                frame.with(&manager, |_| Ok(()))?;
                Ok(())
            })() {
                Ok(()) => {}
                Err(err) if frame.matches(&err, 122, || m.global("TypeError"))? => {
                    let e = frame.catch(err);
                    let _handling = rt::Handling::new(&e);
                    frame.line(123);
                    m.global("print")?.call1(("TypeError:", &e))?;
                }
                Err(err) => return Err(err),
            }
            py.check_signals()?;
            frame.line(118);
        }
        Ok(rt::none(py))
    })
}

// with_stmt.py:126
pub fn lookup_on_type<'py>(module: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(module, "lookup_on_type", 126, |frame| {
        frame.line(127);
        let m = module.global("InstanceMethods")?.call0()?;
        // with_stmt.py:128
        fn lambda<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
            Ok(rt::none(m.py()))
        }
        frame.line(128);
        m.setattr(
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
        frame.line(129);
        m.setattr(
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
            Err(err) if frame.matches(&err, 133, || module.global("TypeError"))? => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                frame.line(134);
                module.global("print")?.call1(("TypeError:", &e))?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(module.py()))
    })
}

// with_stmt.py:137
pub fn suppress_in_loop<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "suppress_in_loop", 137, |frame| {
        let mut count = rt::int(py, 0);
        frame.line(139);
        for i in m.global("range")?.call1((3,))?.try_iter()? {
            let i = i?;
            frame.line(140);
            frame.with(
                &rt::call!(
                    m.global("Manager")?,
                    m.global("str")?.call1((&i,))?,
                    suppress = true
                )?,
                |_| {
                    frame.line(141);
                    if i.eq(1)? {
                        frame.line(142);
                        return Err(frame.raise(m.global("ValueError")?.call1((&i,))?));
                    }
                    frame.line(143);
                    count = rt::iadd(&count, 1)?;
                    Ok(())
                },
            )?;
            py.check_signals()?;
            frame.line(139);
        }
        Ok(count)
    })
}

pub mod manager {
    use super::*;

    // with_stmt.py:7
    /// The body of `class Manager`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Manager", 7, |frame| {
            frame.line(8);
            ns.set("__init__", rt::function!(
    ns,
    __init__(slf as "self", name, suppress = false, fail_enter = false, fail_exit = false),
    qualname = "Manager.__init__"
)?)?;
            frame.line(14);
            ns.set(
                "__enter__",
                rt::function!(
                    ns,
                    __enter__(slf as "self"),
                    qualname = "Manager.__enter__"
                )?,
            )?;
            frame.line(20);
            ns.set(
                "__exit__",
                rt::function!(
                    ns,
                    __exit__(slf as "self", exc_type, exc, tb),
                    qualname = "Manager.__exit__"
                )?,
            )?;
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
        rt::frame!(m, "Manager.__init__", 8, |frame| {
            frame.line(9);
            slf.setattr("name", name)?;
            frame.line(10);
            slf.setattr("suppress", suppress)?;
            frame.line(11);
            slf.setattr("fail_enter", fail_enter)?;
            frame.line(12);
            slf.setattr("fail_exit", fail_exit)?;
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
            frame.line(15);
            m.global("print")?.call1(("enter", slf.getattr("name")?))?;
            frame.line(16);
            if slf.getattr("fail_enter")?.is_truthy()? {
                frame.line(17);
                return Err(frame.raise(
                    m.global("RuntimeError")?
                        .call1((rt::str(py, "enter failed: ").add(slf.getattr("name")?)?,))?,
                ));
            }
            frame.line(18);
            Ok(PyTuple::new(
                py,
                [
                    &slf.getattr("name")?,
                    &m.global("len")?.call1((slf.getattr("name")?,))?,
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
            frame.line(21);
            m.global("print")?.call1((
                "exit",
                slf.getattr("name")?,
                if exc_type.is_truthy()? {
                    exc_type.getattr("__name__")?
                } else {
                    rt::none(py)
                },
                m.global("repr")?.call1((exc,))?,
                !tb.is_none(),
                "handled:",
                m.global("repr")?
                    .call1((m.global("sys")?.call_method0("exception")?,))?,
            ))?;
            frame.line(23);
            if slf.getattr("fail_exit")?.is_truthy()? {
                frame.line(24);
                return Err(frame.raise(
                    m.global("KeyError")?
                        .call1((rt::str(py, "exit failed: ").add(slf.getattr("name")?)?,))?,
                ));
            }
            frame.line(25);
            slf.getattr("suppress")
        })
    }
}

pub mod no_exit {
    use super::*;

    // with_stmt.py:28
    /// The body of `class NoExit`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "NoExit", 28, |frame| {
            frame.line(29);
            ns.set(
                "__enter__",
                rt::function!(
                    ns,
                    __enter__(slf as "self"),
                    qualname = "NoExit.__enter__"
                )?,
            )?;
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
            // with_stmt.py:34, kept as Python: async function at 34:5
            rt::exec_python!(
                ns,
                line = 34,
                r#"
    async def __aenter__(self):
        return self
"#
            )?;
            // with_stmt.py:37, kept as Python: async function at 37:5
            rt::exec_python!(
                ns,
                line = 37,
                r#"
    async def __aexit__(self, *args):
        return False
"#
            )?;
            Ok(())
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
        frame.line(2);
        m.set_global("os", rt::import(m, "os")?)?;
        frame.line(3);
        m.set_global("sys", rt::import(m, "sys")?)?;
        frame.line(4);
        m.set_global("tempfile", rt::import(m, "tempfile")?)?;
        frame.line(7);
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
        frame.line(28);
        m.set_global(
            "NoExit",
            rt::class!(m, "NoExit"(), no_exit::body, line = 28)?,
        )?;
        frame.line(33);
        m.set_global("Async", rt::class!(m, "Async"(), async_::body, line = 33)?)?;
        frame.line(41);
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
        frame.line(147);
        m.set_global(
            "path",
            m.global("os")?
                .getattr("path")?
                .getattr("join")?
                .call1((m.global("tempfile")?.call_method0("mkdtemp")?, "data.txt"))?,
        )?;
        frame.line(148);
        m.global("print")?
            .call1((m.global("write_and_read")?.call1((m.global("path")?,))?,))?;
        frame.line(149);
        m.global("target_kinds")?.call0()?;
        frame.line(150);
        m.global("multiple")?.call0()?;
        frame.line(151);
        m.global("suppressing")?.call0()?;
        frame.line(152);
        m.global("not_suppressing")?.call0()?;
        frame.line(153);
        m.global("enter_fails")?.call0()?;
        frame.line(154);
        m.global("exit_fails")?.call0()?;
        frame.line(155);
        m.global("print")?.call1((m
            .global("jumps")?
            .call1((PyList::new(py, ["a", "skip", "b", "stop", "c"])?,))?,))?;
        frame.line(156);
        m.global("print")?.call1((m
            .global("jumps")?
            .call1((PyList::new(py, ["a", "ret"])?,))?,))?;
        frame.line(157);
        m.global("print")?
            .call1((m.global("returns_value")?.call0()?,))?;
        frame.line(158);
        m.global("bad_managers")?.call0()?;
        frame.line(159);
        m.global("lookup_on_type")?.call0()?;
        frame.line(160);
        m.global("print")?
            .call1((m.global("suppress_in_loop")?.call0()?,))?;
        frame.line(161);
        frame.with(&m.global("Manager")?.call1(("module",))?, |value| {
            m.set_global("pair", value)?;
            frame.line(162);
            m.global("print")?
                .call1(("module level", m.global("pair")?))?;
            Ok(())
        })?;
        frame.line(163);
        frame.with(
            &rt::call!(m.global("Manager")?, "module quiet", suppress = true)?,
            |_| -> PyResult<()> {
                frame.line(164);
                Err(frame.raise(m.global("ValueError")?.call1(("module",))?))
            },
        )?;
        frame.line(165);
        m.global("print")?.call1((
            m.globals().clone().into_any().contains("pair")?,
            m.global("pair")?,
        ))?;
        Ok(())
    })
}
