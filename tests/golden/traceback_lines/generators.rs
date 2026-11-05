use pyo3::exceptions::PyKeyError;
use pyo3::exceptions::PyStopIteration;
use pyo3::exceptions::PyValueError;
use pyo3::exceptions::PyZeroDivisionError;
use pyo3::prelude::*;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// generators.py:7
pub async fn count<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "count", 7, async |frame| {
        let mut i = rt::int(py, 0);
        frame.line(9);
        while i.lt(n)? {
            frame.line(10);
            rt::yield_(&i).await?;
            frame.line(11);
            i = rt::iadd(&i, 1)?;
            py.check_signals()?;
            frame.line(9);
        }
        Ok(rt::none(py))
    })
}

// generators.py:14
pub async fn averager<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "averager", 14, async |frame| {
        let mut total = rt::int(py, 0);
        let mut n = rt::int(py, 0);
        let mut average = rt::none(py);
        let mut value: Bound<'py, PyAny>;
        loop {
            frame.line(19);
            value = rt::yield_(&average).await?;
            if value.is_none() {
                return Ok(total);
            }
            frame.line(22);
            total = rt::iadd(&total, &value)?;
            frame.line(23);
            n = rt::iadd(&n, 1)?;
            frame.line(24);
            average = total.div(&n)?;
            py.check_signals()?;
        }
    })
}

// generators.py:27
pub async fn bare<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "bare", 27, async |frame| {
        frame.line(28);
        rt::yield_(rt::none(py)).await?;
        frame.line(29);
        let x = rt::yield_(rt::none(py)).await?;
        frame.line(30);
        rt::builtins::print(py, ("bare received", &x))?;
        Ok(rt::none(py))
    })
}

// generators.py:33
pub async fn with_cleanup<'py>(
    m: &rt::Module<'py>,
    log: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "with_cleanup", 33, async |frame| {
        let result: PyResult<()> = async {
            frame.line(35);
            rt::yield_(rt::str(py, "first")).await?;
            frame.line(36);
            rt::yield_(rt::str(py, "second")).await?;
            Ok(())
        }
        .await;
        let finally = frame.finally(result);
        frame.line(38);
        log.method1("append", ("cleanup",))?;
        finally.resume()?;
        Ok(rt::none(py))
    })
}

// generators.py:41
pub async fn catches<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "catches", 41, async |frame| {
        loop {
            frame.line(44);
            match rt::yield_(rt::str(py, "ready")).await {
                Ok(_) => {}
                Err(err) if err.is_instance_of::<PyValueError>(py) => {
                    let e = frame.catch(err);
                    frame.line(46);
                    rt::builtins::print(
                        py,
                        ("caught", &e, m.global("sys")?.method0("exception")?.repr()?),
                    )?;
                    frame.line(47);
                    rt::yield_(rt::str(py, "recovered")).await?;
                }
                Err(err) => return Err(err),
            }
            py.check_signals()?;
        }
    })
}

// generators.py:50
pub async fn in_loops<'py>(
    m: &rt::Module<'py>,
    rows: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "in_loops", 50, async |frame| {
        frame.line(51);
        for row in rows.try_iter()? {
            let row = row?;
            if row.is_none() {
                py.check_signals()?;
                continue;
            }
            frame.line(54);
            'for_else: {
                for item in row.try_iter()? {
                    let item = item?;
                    frame.line(55);
                    if item.eq("stop")? {
                        break 'for_else;
                    }
                    frame.line(57);
                    if item.eq("end")? {
                        return Ok(rt::str(py, "ended"));
                    }
                    frame.line(59);
                    rt::yield_(&item).await?;
                    py.check_signals()?;
                    frame.line(54);
                }
                frame.line(61);
                rt::yield_(rt::str(py, "row done")).await?;
            }
            py.check_signals()?;
            frame.line(51);
        }
        Ok(rt::str(py, "all done"))
    })
}

// generators.py:65
pub async fn in_with<'py>(
    m: &rt::Module<'py>,
    manager: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "in_with", 65, async |frame| {
        frame
            .with_async(manager, async |value| {
                frame.line(67);
                rt::yield_(value).await?;
                frame.line(68);
                rt::yield_(value.mul(2)?).await?;
                Ok(())
            })
            .await?;
        Ok(rt::none(m.py()))
    })
}

// generators.py:84
pub async fn python_delegate<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "python_delegate", 84, async |frame| {
        frame.line(85);
        let iter = PyList::new(py, [1, 2])?.try_iter()?;
        let mut result = rt::yield_from(iter).await?;
        frame.line(86);
        rt::builtins::print(py, ("python delegate result", &result))?;
        frame.line(87);
        result = rt::delegate(averager(m)).await?;
        Ok(result)
    })
}

// generators.py:91
pub async fn native_delegate<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "native_delegate", 91, async |frame| {
        frame.line(92);
        let total = rt::delegate(averager(m)).await?;
        frame.line(93);
        rt::builtins::print(py, ("native delegate total", &total))?;
        frame.line(94);
        let rest = rt::delegate(count(m, &rt::int(py, 2))).await?;
        frame.line(95);
        rt::builtins::print(py, ("count returned", &rest))?;
        frame.line(96);
        rt::delegate(in_loops(
            m,
            PyList::new(
                py,
                [PyList::new(py, [&rt::int(py, 1), &rt::str(py, "end")])?.as_any()],
            )?
            .as_any(),
        ))
        .await?;
        Ok(total)
    })
}

// generators.py:100
/// Recursive `yield from` of the same native generator.
pub async fn tree<'py>(
    m: &rt::Module<'py>,
    node: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "tree", 100, async |frame| {
        frame.line(102);
        if node.is_instance(&rt::builtin(py, "list")?)? {
            frame.line(103);
            for child in node.try_iter()? {
                let child = child?;
                frame.line(104);
                rt::delegate(Box::pin(tree(m, &child))).await?;
                py.check_signals()?;
                frame.line(103);
            }
        } else {
            frame.line(106);
            rt::yield_(node).await?;
        }
        Ok(rt::none(py))
    })
}

// generators.py:109
pub async fn ping<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "ping", 109, async |frame| {
        frame.line(110);
        if n.is_truthy()? {
            frame.line(111);
            let value = PyTuple::new(py, [&rt::str(py, "ping"), n])?;
            rt::yield_(value).await?;
            frame.line(112);
            rt::delegate(Box::pin(pong(m, &n.sub(1)?))).await?;
        }
        Ok(rt::none(py))
    })
}

// generators.py:115
pub async fn pong<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "pong", 115, async |frame| {
        frame.line(116);
        if n.is_truthy()? {
            frame.line(117);
            let value = PyTuple::new(py, [&rt::str(py, "pong"), n])?;
            rt::yield_(value).await?;
            frame.line(118);
            rt::delegate(Box::pin(ping(m, &n.sub(1)?))).await?;
        }
        Ok(rt::none(py))
    })
}

// generators.py:148
pub async fn tag<'py>(
    m: &rt::Module<'py>,
    name: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "tag", 147, async |frame| {
        frame.line(149);
        rt::builtins::print(py, (rt::fstring(py, ("<", rt::format(name, "")?, ">"))?,))?;
        let result: PyResult<()> = async {
            frame.line(151);
            match async { rt::yield_(name.method0("upper")?).await }.await {
                Ok(_) => {}
                Err(err) if err.is_instance_of::<PyZeroDivisionError>(py) => {
                    let _caught = frame.catch(err);
                    frame.line(153);
                    rt::builtins::print(py, ("contextmanager caught",))?;
                }
                Err(err) => return Err(err),
            }
            Ok(())
        }
        .await;
        let finally = frame.finally(result);
        frame.line(155);
        rt::builtins::print(py, (rt::fstring(py, ("</", rt::format(name, "")?, ">"))?,))?;
        finally.resume()?;
        Ok(rt::none(py))
    })
}

// generators.py:158
pub fn make_counter<'py>(
    m: &rt::Module<'py>,
    start: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "make_counter", 158, |frame| {
        frame.line(159);
        let count = rt::Cell::new(m.py(), "count", start)?;
        // generators.py:161
        async fn counter<'py>(
            m: &rt::Module<'py>,
            count: &rt::Cell<'py>,
        ) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "make_counter.<locals>.counter", 161, async |frame| {
                loop {
                    frame.line(164);
                    count.set(rt::iadd(&count.get()?, 1)?)?;
                    frame.line(165);
                    rt::yield_(count.get()?).await?;
                    m.py().check_signals()?;
                }
            })
        }
        let counter = rt::function!(
            m,
            gen counter[&count](),
            qualname = "make_counter.<locals>.counter"
        )?;
        frame.line(167);
        counter.call0()
    })
}

// generators.py:170
pub fn closure_gen<'py>(
    m: &rt::Module<'py>,
    values: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "closure_gen", 170, |frame| {
        let scale = rt::int(m.py(), 10);
        // generators.py:172
        fn lambda<'py>(
            m: &rt::Module<'py>,
            values: &Bound<'py, PyAny>,
            scale: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            let py = m.py();
            rt::frame!(m, "closure_gen.<locals>.<lambda>", 172, |_| {
                let list = PyList::empty(py);
                for v in values.try_iter()? {
                    let v = v?;
                    list.append(v.mul(scale)?)?;
                    py.check_signals()?;
                }
                Ok(list.into_any())
            })
        }
        frame.line(172);
        rt::function!(
            m,
            lambda[values, scale](),
            qualname = "closure_gen.<locals>.<lambda>"
        )?
        .call0()
    })
}

// generators.py:175
pub async fn gen_with_nested<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "gen_with_nested", 175, async |frame| {
        // generators.py:176
        async fn inner<'py>(
            m: &rt::Module<'py>,
            x: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "gen_with_nested.<locals>.inner", 176, async |frame| {
                frame.line(177);
                rt::yield_(x).await?;
                frame.line(178);
                rt::yield_(x.add(1)?).await?;
                Ok(rt::none(m.py()))
            })
        }
        let inner = rt::function!(
            m,
            gen inner(x),
            qualname = "gen_with_nested.<locals>.inner"
        )?;
        frame.line(180);
        for value in inner.call1((5,))?.try_iter()? {
            let value = value?;
            frame.line(181);
            rt::yield_(&value).await?;
            py.check_signals()?;
            frame.line(180);
        }
        Ok(rt::none(py))
    })
}

pub mod manager {
    use super::*;

    // generators.py:71
    /// The body of `class Manager`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Manager", 71, |_| {
            rt::def!(ns, __init__(slf as "self", value))?;
            rt::def!(ns, __enter__(slf as "self"))?;
            rt::def!(ns, __exit__(slf as "self", typ, exc, tb))?;
            Ok(())
        })
    }

    // generators.py:72
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        value: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Manager.__init__", 72, |frame| {
            frame.line(73);
            slf.set_attr("value", value)?;
            Ok(rt::none(m.py()))
        })
    }

    // generators.py:75
    pub fn __enter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Manager.__enter__", 75, |frame| {
            frame.line(76);
            rt::builtins::print(m.py(), ("enter", slf.attr("value")?))?;
            frame.line(77);
            slf.attr("value")
        })
    }

    // generators.py:79
    pub fn __exit__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        typ: &Bound<'py, PyAny>,
        exc: &Bound<'py, PyAny>,
        tb: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Manager.__exit__", 79, |frame| {
            frame.line(80);
            rt::builtins::print(
                py,
                (
                    "exit",
                    if typ.is_truthy()? {
                        typ.attr("__name__")?
                    } else {
                        rt::none(py)
                    },
                ),
            )?;
            frame.line(81);
            Ok(rt::bool(py, typ.is(rt::builtin(py, "KeyError")?)))
        })
    }
}

pub mod bag {
    use super::*;

    // generators.py:121
    /// The body of `class Bag`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Bag", 121, |frame| {
            rt::def!(ns, __init__(slf as "self", *items))?;
            rt::def!(ns, gen __iter__(slf as "self"))?;
            rt::def!(ns, gen pairs(slf as "self"))?;
            frame.line(136);
            rt::def!(ns, @property doubled(slf as "self"))?;
            Ok(())
        })
    }

    // generators.py:122
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        items: &Bound<'py, PyTuple>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Bag.__init__", 122, |frame| {
            frame.line(123);
            slf.set_attr("items", items)?;
            Ok(rt::none(m.py()))
        })
    }

    // generators.py:125
    pub async fn __iter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Bag.__iter__", 125, async |frame| {
            frame.line(126);
            for item in slf.attr("items")?.try_iter()? {
                let item = item?;
                frame.line(127);
                rt::yield_(&item).await?;
                py.check_signals()?;
                frame.line(126);
            }
            Ok(rt::none(py))
        })
    }

    // generators.py:129
    pub async fn pairs<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Bag.pairs", 129, async |frame| {
            let mut previous = rt::none(py);
            frame.line(131);
            for item in slf.try_iter()? {
                let item = item?;
                if !previous.is_none() {
                    frame.line(133);
                    rt::yield_(PyTuple::new(py, [&previous, &item])?).await?;
                }
                previous = item.clone();
                py.check_signals()?;
                frame.line(131);
            }
            Ok(rt::none(py))
        })
    }

    // generators.py:137
    pub fn doubled<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Bag.doubled", 136, |frame| {
            // generators.py:138
            fn genexpr<'py>(
                m: &rt::Module<'py>,
                iter: &Bound<'py, PyIterator>,
            ) -> PyResult<Option<Bound<'py, PyAny>>> {
                rt::frame!(m, "Bag.doubled.<locals>.<genexpr>", 138, |_| {
                    if let Some(x) = rt::next(iter)? {
                        return Ok(Some(x.mul(2)?));
                    }
                    Ok(None)
                })
            }
            frame.line(138);
            rt::genexpr!(
                m,
                genexpr(slf.try_iter()?),
                qualname = "Bag.doubled.<locals>.<genexpr>"
            )
        })
    }
}

pub mod sub {
    use super::*;

    // generators.py:141
    /// The body of `class Sub`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Sub", 141, |_| {
            let __class__ = ns.class_cell();
            rt::def!(ns, gen __iter__[&__class__](slf as "self"))?;
            Ok(())
        })
    }

    // generators.py:142
    pub async fn __iter__<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Sub.__iter__", 142, async |frame| {
            frame.line(143);
            rt::yield_(rt::str(py, "sub")).await?;
            frame.line(144);
            let __iter__ = rt::super_(__class__, slf)?.method0("__iter__")?;
            rt::yield_from(__iter__).await?;
            Ok(rt::none(py))
        })
    }
}

/// Executes the body of `generators.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "Generator functions lowered to native `async fn`s (design §8).",
        )?;
        frame.line(3);
        m.set_global("contextlib", rt::import(m, "contextlib")?)?;
        frame.line(4);
        m.set_global("sys", rt::import(m, "sys")?)?;
        rt::def!(m, gen count(n))?;
        rt::def!(m, gen averager())?;
        rt::def!(m, gen bare())?;
        rt::def!(m, gen with_cleanup(log))?;
        rt::def!(m, gen catches())?;
        rt::def!(m, gen in_loops(rows))?;
        rt::def!(m, gen in_with(manager))?;
        frame.line(71);
        m.set_global(
            "Manager",
            rt::class!(
                m,
                "Manager"(),
                manager::body,
                line = 71,
                attributes = ["value"]
            )?,
        )?;
        rt::def!(m, gen python_delegate())?;
        rt::def!(m, gen native_delegate())?;
        rt::def!(
            m,
            gen tree(node),
            doc = "Recursive `yield from` of the same native generator."
        )?;
        rt::def!(m, gen ping(n))?;
        rt::def!(m, gen pong(n))?;
        frame.line(121);
        m.set_global(
            "Bag",
            rt::class!(m, "Bag"(), bag::body, line = 121, attributes = ["items"])?,
        )?;
        frame.line(141);
        m.set_global(
            "Sub",
            rt::class!(m, "Sub"(m.global("Bag")?), sub::body, line = 141)?,
        )?;
        frame.line(147);
        rt::def!(m, @contextlib.contextmanager gen tag(name))?;
        rt::def!(m, make_counter(start))?;
        rt::def!(m, closure_gen(values))?;
        rt::def!(m, gen gen_with_nested())?;
        frame.line(184);
        rt::builtins::print(
            py,
            (rt::builtin(py, "list")?.call1((m.global("count")?.call1((4,))?,))?,),
        )?;
        frame.line(185);
        m.set_global("g", m.global("averager")?.call0()?)?;
        frame.line(186);
        rt::builtins::print(
            py,
            (
                rt::builtin(py, "next")?.call1((m.global("g")?,))?,
                m.global("g")?.method1("send", (10,))?,
                m.global("g")?.method1("send", (20,))?,
            ),
        )?;
        frame.line(188);
        match (|| m.global("g")?.method1("send", (py.None(),)))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                frame.line(190);
                rt::builtins::print(py, ("returned", m.global("e")?.attr("value")?))?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        frame.line(192);
        m.set_global("b", m.global("bare")?.call0()?)?;
        frame.line(193);
        rt::builtins::print(
            py,
            (
                rt::builtin(py, "next")?.call1((m.global("b")?,))?,
                rt::builtin(py, "next")?.call1((m.global("b")?,))?,
            ),
        )?;
        frame.line(195);
        match (|| m.global("b")?.method1("send", ("hello",)))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
                let _caught = frame.catch(err);
                frame.line(197);
                rt::builtins::print(py, ("bare done",))?;
            }
            Err(err) => return Err(err),
        }
        frame.line(199);
        m.set_global("log", PyList::empty(py))?;
        frame.line(200);
        m.set_global("c", m.global("with_cleanup")?.call1((m.global("log")?,))?)?;
        frame.line(201);
        rt::builtins::print(py, (rt::builtin(py, "next")?.call1((m.global("c")?,))?,))?;
        frame.line(202);
        m.global("c")?.method0("close")?;
        frame.line(203);
        rt::builtins::print(py, ("closed", m.global("log")?))?;
        frame.line(204);
        m.set_global("c", m.global("with_cleanup")?.call1((m.global("log")?,))?)?;
        frame.line(205);
        rt::builtins::print(
            py,
            (
                rt::builtin(py, "next")?.call1((m.global("c")?,))?,
                rt::builtin(py, "next")?.call1((m.global("c")?,))?,
            ),
        )?;
        frame.line(207);
        match (|| rt::builtin(py, "next")?.call1((m.global("c")?,)))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
                let _caught = frame.catch(err);
                frame.line(209);
                rt::builtins::print(py, ("exhausted", m.global("log")?))?;
            }
            Err(err) => return Err(err),
        }
        frame.line(211);
        m.set_global("r", m.global("catches")?.call0()?)?;
        frame.line(212);
        rt::builtins::print(py, (rt::builtin(py, "next")?.call1((m.global("r")?,))?,))?;
        frame.line(213);
        rt::builtins::print(
            py,
            (m.global("r")?
                .attr("throw")?
                .call1((rt::builtin(py, "ValueError")?.call1(("boom",))?,))?,),
        )?;
        frame.line(214);
        rt::builtins::print(py, (rt::builtin(py, "next")?.call1((m.global("r")?,))?,))?;
        frame.line(216);
        match (|| {
            m.global("r")?
                .attr("throw")?
                .call1((rt::builtin(py, "KeyError")?.call1(("other",))?,))
        })() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyKeyError>(py) => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                frame.line(218);
                rt::builtins::print(py, ("propagated", m.global("e")?.repr()?))?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        frame.line(219);
        rt::builtins::print(py, (m.global("sys")?.method0("exception")?.repr()?,))?;
        frame.line(221);
        m.set_global(
            "loops",
            m.global("in_loops")?.call1((PyList::new(
                py,
                [
                    PyList::new(py, [1, 2])?.as_any(),
                    &rt::none(py),
                    PyList::new(py, [&rt::int(py, 3), &rt::str(py, "stop"), &rt::int(py, 4)])?
                        .as_any(),
                    PyList::new(py, [5])?.as_any(),
                ],
            )?,))?,
        )?;
        frame.line(222);
        rt::builtins::print(
            py,
            (rt::builtin(py, "list")?.call1((m.global("loops")?,))?,),
        )?;
        frame.line(224);
        match (|| rt::builtin(py, "next")?.call1((m.global("loops")?,)))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                frame.line(226);
                rt::builtins::print(py, ("loops stopped", m.global("e")?.attr("value")?))?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        frame.line(227);
        rt::builtins::print(
            py,
            (
                rt::builtin(py, "list")?.call1((m.global("in_loops")?.call1((PyList::new(
                    py,
                    [
                        PyList::new(py, [&rt::int(py, 1), &rt::str(py, "end"), &rt::int(py, 2)])?
                            .as_any(),
                    ],
                )?,))?,))?,
            ),
        )?;
        frame.line(229);
        m.set_global(
            "w",
            m.global("in_with")?
                .call1((m.global("Manager")?.call1((3,))?,))?,
        )?;
        frame.line(230);
        rt::builtins::print(py, (rt::builtin(py, "list")?.call1((m.global("w")?,))?,))?;
        frame.line(231);
        m.set_global(
            "w",
            m.global("in_with")?
                .call1((m.global("Manager")?.call1((4,))?,))?,
        )?;
        frame.line(232);
        rt::builtins::print(py, (rt::builtin(py, "next")?.call1((m.global("w")?,))?,))?;
        frame.line(233);
        m.global("w")?.method0("close")?;
        frame.line(234);
        m.set_global(
            "w",
            m.global("in_with")?
                .call1((m.global("Manager")?.call1((5,))?,))?,
        )?;
        frame.line(235);
        rt::builtin(py, "next")?.call1((m.global("w")?,))?;
        frame.line(237);
        match (|| {
            m.global("w")?
                .attr("throw")?
                .call1((rt::builtin(py, "KeyError")?.call1(("suppressed",))?,))
        })() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
                let _caught = frame.catch(err);
                frame.line(239);
                rt::builtins::print(py, ("suppressed by __exit__",))?;
            }
            Err(err) => return Err(err),
        }
        frame.line(241);
        m.set_global("p", m.global("python_delegate")?.call0()?)?;
        frame.line(242);
        rt::builtins::print(
            py,
            (
                rt::builtin(py, "next")?.call1((m.global("p")?,))?,
                rt::builtin(py, "next")?.call1((m.global("p")?,))?,
                rt::builtin(py, "next")?.call1((m.global("p")?,))?,
                m.global("p")?.method1("send", (4,))?,
                m.global("p")?.method1("send", (6,))?,
            ),
        )?;
        frame.line(244);
        match (|| m.global("p")?.method1("send", (py.None(),)))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                frame.line(246);
                rt::builtins::print(
                    py,
                    ("python_delegate returned", m.global("e")?.attr("value")?),
                )?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        frame.line(248);
        m.set_global("n", m.global("native_delegate")?.call0()?)?;
        frame.line(249);
        rt::builtins::print(
            py,
            (
                rt::builtin(py, "next")?.call1((m.global("n")?,))?,
                m.global("n")?.method1("send", (1,))?,
                m.global("n")?.method1("send", (3,))?,
            ),
        )?;
        frame.line(250);
        rt::builtins::print(
            py,
            (
                m.global("n")?.method1("send", (py.None(),))?,
                rt::builtin(py, "next")?.call1((m.global("n")?,))?,
                rt::builtin(py, "next")?.call1((m.global("n")?,))?,
            ),
        )?;
        frame.line(252);
        match (|| rt::builtin(py, "next")?.call1((m.global("n")?,)))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                frame.line(254);
                rt::builtins::print(
                    py,
                    ("native_delegate returned", m.global("e")?.attr("value")?),
                )?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        frame.line(256);
        rt::builtins::print(
            py,
            (
                rt::builtin(py, "list")?.call1((m.global("tree")?.call1((PyList::new(
                    py,
                    [
                        &rt::int(py, 1),
                        PyList::new(py, [&rt::int(py, 2), PyList::new(py, [3, 4])?.as_any()])?
                            .as_any(),
                        PyList::empty(py).as_any(),
                        &rt::int(py, 5),
                    ],
                )?,))?,))?,
            ),
        )?;
        frame.line(257);
        rt::builtins::print(
            py,
            (rt::builtin(py, "list")?.call1((m.global("ping")?.call1((4,))?,))?,),
        )?;
        frame.line(259);
        m.set_global("bag", m.global("Bag")?.call1((1, 2, 3))?)?;
        frame.line(260);
        rt::builtins::print(
            py,
            (
                rt::builtin(py, "list")?.call1((m.global("bag")?,))?,
                rt::builtin(py, "list")?.call1((m.global("bag")?.method0("pairs")?,))?,
                rt::builtin(py, "list")?.call1((m.global("bag")?.attr("doubled")?,))?,
            ),
        )?;
        frame.line(261);
        rt::builtins::print(
            py,
            (rt::builtin(py, "list")?.call1((m.global("Sub")?.call1((7, 8))?,))?,),
        )?;
        frame.line(263);
        frame.with(&m.global("tag")?.call1(("a",))?, |value| {
            m.set_global("name", value)?;
            frame.line(264);
            rt::builtins::print(py, ("inside", m.global("name")?))?;
            Ok(())
        })?;
        frame.line(265);
        frame.with(&m.global("tag")?.call1(("b",))?, |_| {
            frame.line(266);
            rt::int(py, 1).div(0)?;
            Ok(())
        })?;
        frame.line(267);
        rt::builtins::print(py, ("after b",))?;
        frame.line(269);
        m.set_global("counter", make_counter(m, &rt::int(py, 5))?)?;
        frame.line(270);
        rt::builtins::print(
            py,
            (
                rt::builtin(py, "next")?.call1((m.global("counter")?,))?,
                rt::builtin(py, "next")?.call1((m.global("counter")?,))?,
            ),
        )?;
        frame.line(271);
        rt::builtins::print(py, (closure_gen(m, PyList::new(py, [1, 2])?.as_any())?,))?;
        frame.line(272);
        rt::builtins::print(
            py,
            (rt::builtin(py, "list")?.call1((m.global("gen_with_nested")?.call0()?,))?,),
        )?;
        frame.line(274);
        m.set_global("gen", m.global("count")?.call1((2,))?)?;
        frame.line(275);
        rt::builtins::print(
            py,
            (
                m.global("gen")?.get_type().attr("__name__")?,
                m.global("gen")?.attr("__name__")?,
                m.global("gen")?.attr("__qualname__")?,
            ),
        )?;
        frame.line(276);
        rt::builtins::print(
            py,
            (
                rt::builtin(py, "next")?.call1((m.global("gen")?,))?,
                m.global("gen")?.attr("gi_running")?,
                m.global("gen")?.attr("gi_suspended")?,
            ),
        )?;
        frame.line(277);
        rt::builtins::print(
            py,
            (
                rt::builtin(py, "list")?.call1((m.global("gen")?,))?,
                m.global("gen")?.attr("gi_suspended")?,
            ),
        )?;
        Ok(())
    })
}
