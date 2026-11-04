use pyo3::exceptions::PyBaseException;
use pyo3::exceptions::PyGeneratorExit;
use pyo3::exceptions::PyKeyError;
use pyo3::exceptions::PyLookupError;
use pyo3::exceptions::PyRuntimeError;
use pyo3::exceptions::PyStopIteration;
use pyo3::exceptions::PyTypeError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// generator_close.py:9
pub fn via<'py>(m: &rt::Module<'py>, f: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    Ok(f.clone())
}

// generator_close.py:13
pub async fn swallows<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "swallows", 13, async |frame| {
        let result: PyResult<rt::Flow<'py>> = async {
            match rt::yield_(rt::int(py, 1)).await {
                Ok(_) => {}
                Err(err) if err.is_instance_of::<PyGeneratorExit>(py) => {
                    let _caught = frame.catch(err);
                    rt::builtins::print(py, ("  swallows: caught GeneratorExit",))?;
                    return Ok(rt::Flow::Return(rt::str(py, "swallowed")));
                }
                Err(err) => return Err(err),
            }
            Ok(rt::Flow::Next)
        }
        .await;
        let finally = frame.finally(result);
        rt::builtins::print(py, ("  swallows: finally",))?;
        if let rt::Flow::Return(value) = finally.resume()? {
            return Ok(value);
        }
        rt::yield_(rt::str(py, "swallows continues")).await?;
        Ok(rt::none(py))
    })
}

// generator_close.py:24
pub async fn outer<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "outer", 24, async |frame| {
        let result2: PyResult<()> = async {
            let result: PyResult<()> = async {
                let r = rt::delegate(swallows(m)).await?;
                rt::builtins::print(py, ("  outer continues with", &r))?;
                rt::yield_(rt::str(py, "outer-after")).await?;
                Ok(())
            }
            .await;
            match result {
                Ok(()) => {}
                Err(err) if err.is_instance_of::<PyGeneratorExit>(py) => {
                    let _caught = frame.catch(err);
                    rt::builtins::print(py, ("  outer: caught GeneratorExit",))?;
                    return Err(frame.reraise());
                }
                Err(err) => return Err(err),
            }
            Ok(())
        }
        .await;
        let finally = frame.finally(result2);
        rt::builtins::print(py, ("  outer: finally",))?;
        finally.resume()?;
        Ok(rt::none(py))
    })
}

// generator_close.py:36
pub async fn outer_via<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "outer_via", 36, async |frame| {
        let result2: PyResult<()> = async {
            let result: PyResult<()> = async {
                let iterable = via(m, &m.global("swallows")?)?.call0()?;
                let r = rt::yield_from(iterable).await?;
                rt::builtins::print(py, ("  outer continues with", &r))?;
                rt::yield_(rt::str(py, "outer-after")).await?;
                Ok(())
            }
            .await;
            match result {
                Ok(()) => {}
                Err(err) if err.is_instance_of::<PyGeneratorExit>(py) => {
                    let _caught = frame.catch(err);
                    rt::builtins::print(py, ("  outer: caught GeneratorExit",))?;
                    return Err(frame.reraise());
                }
                Err(err) => return Err(err),
            }
            Ok(())
        }
        .await;
        let finally = frame.finally(result2);
        rt::builtins::print(py, ("  outer: finally",))?;
        finally.resume()?;
        Ok(rt::none(py))
    })
}

// generator_close.py:48
pub async fn yields_again<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "yields_again", 48, async |frame| {
        let result: PyResult<()> = async {
            match rt::yield_(rt::int(py, 1)).await {
                Ok(_) => {}
                Err(err) if err.is_instance_of::<PyGeneratorExit>(py) => {
                    let _caught = frame.catch(err);
                    rt::builtins::print(py, ("  yields_again: yields again",))?;
                    rt::yield_(rt::str(py, "again")).await?;
                }
                Err(err) => return Err(err),
            }
            Ok(())
        }
        .await;
        let finally = frame.finally(result);
        rt::builtins::print(py, ("  yields_again: finally",))?;
        finally.resume()?;
        Ok(rt::none(py))
    })
}

// generator_close.py:58
pub async fn outer_of_yields_again<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "outer_of_yields_again", 58, async |frame| {
        match Err(frame.raise(PyKeyError::new_err("outer's"))) {
            Ok(()) => unreachable!(),
            Err(err) if err.is_instance_of::<PyKeyError>(py) => {
                let _caught = frame.catch(err);
                let result: PyResult<()> = async {
                    match rt::delegate(yields_again(m)).await {
                        Ok(_) => {}
                        Err(err) if err.is_instance_of::<PyRuntimeError>(py) => {
                            let e = frame.catch(err);
                            rt::builtins::print(
                                py,
                                (
                                    "  outer caught",
                                    e.repr()?,
                                    "context",
                                    e.attr("__context__")?.repr()?,
                                ),
                            )?;
                            return Err(frame.reraise());
                        }
                        Err(err) => return Err(err),
                    }
                    Ok(())
                }
                .await;
                let finally = frame.finally(result);
                rt::builtins::print(py, ("  outer: finally",))?;
                finally.resume()?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

// generator_close.py:71
pub async fn outer_of_yields_again_via<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "outer_of_yields_again_via", 71, async |frame| {
        match Err(frame.raise(PyKeyError::new_err("outer's"))) {
            Ok(()) => unreachable!(),
            Err(err) if err.is_instance_of::<PyKeyError>(py) => {
                let _caught = frame.catch(err);
                let result: PyResult<()> = async {
                    match async {
                        let iterable = via(m, &m.global("yields_again")?)?.call0()?;
                        rt::yield_from(iterable).await
                    }
                    .await
                    {
                        Ok(_) => {}
                        Err(err) if err.is_instance_of::<PyRuntimeError>(py) => {
                            let e = frame.catch(err);
                            rt::builtins::print(
                                py,
                                (
                                    "  outer caught",
                                    e.repr()?,
                                    "context",
                                    e.attr("__context__")?.repr()?,
                                ),
                            )?;
                            return Err(frame.reraise());
                        }
                        Err(err) => return Err(err),
                    }
                    Ok(())
                }
                .await;
                let finally = frame.finally(result);
                rt::builtins::print(py, ("  outer: finally",))?;
                finally.resume()?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

// generator_close.py:84
pub async fn keeps_yielding<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "keeps_yielding", 84, async |frame| {
        loop {
            match rt::yield_(rt::str(py, "x")).await {
                Ok(_) => {}
                Err(err) if err.is_instance_of::<PyGeneratorExit>(py) => {
                    let _caught = frame.catch(err);
                    rt::builtins::print(py, ("  keeps_yielding: ignores GeneratorExit",))?;
                }
                Err(err) => return Err(err),
            }
            py.check_signals()?;
        }
    })
}

// generator_close.py:92
pub async fn outer_of_keeps_yielding<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "outer_of_keeps_yielding", 92, async |frame| {
        match rt::delegate(keeps_yielding(m)).await {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyRuntimeError>(py) => {
                let e = frame.catch(err);
                rt::builtins::print(py, ("  outer caught", &e))?;
            }
            Err(err) => return Err(err),
        }
        rt::yield_(rt::str(py, "after")).await?;
        Ok(rt::none(py))
    })
}

// generator_close.py:100
pub async fn raises_in_finally<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "raises_in_finally", 100, async |frame| {
        let result: PyResult<()> = async {
            rt::yield_(rt::int(m.py(), 1)).await?;
            Ok(())
        }
        .await;
        let _finally = frame.finally(result);
        Err(frame.raise(PyValueError::new_err("from the delegate's finally")))
    })
}

// generator_close.py:107
pub async fn outer_of_raises<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "outer_of_raises", 107, async |frame| {
        match Err(frame.raise(PyKeyError::new_err("outer's"))) {
            Ok(()) => unreachable!(),
            Err(err) if err.is_instance_of::<PyKeyError>(py) => {
                let _caught = frame.catch(err);
                match rt::delegate(raises_in_finally(m)).await {
                    Ok(_) => {}
                    Err(err) if err.is_instance_of::<PyValueError>(py) => {
                        let e = frame.catch(err);
                        rt::builtins::print(
                            py,
                            (
                                "  outer got",
                                e.repr()?,
                                "context",
                                e.attr("__context__")?.repr()?,
                            ),
                        )?;
                        return Err(frame.reraise());
                    }
                    Err(err) => return Err(err),
                }
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

// generator_close.py:118
pub async fn leaf<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "leaf", 118, async |frame| {
        match rt::yield_(rt::str(py, "leaf")).await {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyGeneratorExit>(py) => {
                let e = frame.catch(err);
                rt::builtins::print(
                    py,
                    ("  leaf: caught, context", e.attr("__context__")?.repr()?),
                )?;
                return Ok(rt::str(py, "leaf-ret"));
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

// generator_close.py:126
pub async fn middle<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "middle", 126, async |frame| {
        let result: PyResult<()> = async {
            let r = rt::delegate(leaf(m)).await?;
            rt::builtins::print(py, ("  middle continues", &r))?;
            rt::yield_(rt::str(py, "middle-after")).await?;
            Ok(())
        }
        .await;
        match result {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyGeneratorExit>(py) => {
                let _caught = frame.catch(err);
                rt::builtins::print(py, ("  middle: caught",))?;
                return Ok(rt::str(py, "middle-ret"));
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

// generator_close.py:136
pub async fn top<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "top", 136, async |frame| {
        let r: Bound<'py, PyAny>;
        match Err(frame.raise(PyKeyError::new_err("top's"))) {
            Ok(()) => unreachable!(),
            Err(err) if err.is_instance_of::<PyKeyError>(py) => {
                let _caught = frame.catch(err);
                r = rt::delegate(middle(m)).await?;
                rt::builtins::print(py, ("  top continues", &r))?;
                rt::yield_(rt::str(py, "top-after")).await?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

// generator_close.py:145
pub async fn countdown<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "countdown", 145, async |frame| {
        let result: PyResult<rt::Flow<'py>> = async {
            if n.eq(0)? {
                rt::yield_(rt::str(py, "bottom")).await?;
                return Ok(rt::Flow::Return(rt::int(py, 0)));
            }
            let r = rt::delegate(Box::pin(countdown(m, &n.sub(1)?))).await?;
            rt::builtins::print(py, ("  countdown continues", n, &r))?;
            rt::yield_(rt::str(py, "after")).await?;
            Ok(rt::Flow::Next)
        }
        .await;
        match result {
            Ok(rt::Flow::Return(value)) => return Ok(value),
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyGeneratorExit>(py) => {
                let _caught = frame.catch(err);
                rt::builtins::print(py, ("  countdown caught", n))?;
                return Ok(n.clone());
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

// generator_close.py:158
pub async fn catches_thrown<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "catches_thrown", 158, async |frame| {
        match rt::yield_(rt::int(py, 1)).await {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyValueError>(py) => {
                let e = frame.catch(err);
                rt::builtins::print(py, ("  delegate: context", e.attr("__context__")?.repr()?))?;
                return Err(frame.raise_from(PyTypeError::new_err("from the delegate"), py.None()));
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

// generator_close.py:166
pub async fn outer_of_catches<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "outer_of_catches", 166, async |frame| {
        match Err(frame.raise(PyKeyError::new_err("outer's"))) {
            Ok(()) => unreachable!(),
            Err(err) if err.is_instance_of::<PyKeyError>(py) => {
                let _caught = frame.catch(err);
                rt::delegate(catches_thrown(m)).await?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

// generator_close.py:178
pub async fn aswallows<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "aswallows", 178, async |frame| {
        match async {
            let sleep = m.global("Sleep")?.call0()?;
            rt::await_(sleep).await
        }
        .await
        {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyGeneratorExit>(py) => {
                let _caught = frame.catch(err);
                rt::builtins::print(py, ("  aswallows: caught GeneratorExit",))?;
                return Ok(rt::str(py, "swallowed"));
            }
            Err(err) => return Err(err),
        }
        Ok(rt::str(py, "normal"))
    })
}

// generator_close.py:187
pub async fn aouter<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "aouter", 187, async |_| {
        let r = rt::delegate(aswallows(m)).await?;
        rt::builtins::print(py, ("  aouter continues with", &r))?;
        let sleep = m.global("Sleep")?.call0()?;
        rt::await_(sleep).await?;
        Ok(rt::str(py, "aouter done"))
    })
}

// generator_close.py:194
pub async fn aouter_via<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "aouter_via", 194, async |_| {
        let awaitable = via(m, &m.global("aswallows")?)?.call0()?;
        let r = rt::await_(awaitable).await?;
        rt::builtins::print(py, ("  aouter continues with", &r))?;
        let sleep = m.global("Sleep")?.call0()?;
        rt::await_(sleep).await?;
        Ok(rt::str(py, "aouter done"))
    })
}

// generator_close.py:201
pub async fn aignores<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "aignores", 201, async |frame| {
        match async {
            let sleep = m.global("Sleep")?.call0()?;
            rt::await_(sleep).await
        }
        .await
        {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyGeneratorExit>(py) => {
                let _caught = frame.catch(err);
                let awaitable = m.global("Sleep")?.call0()?;
                rt::await_(awaitable).await?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

// generator_close.py:208
pub async fn aouter_of_ignores<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "aouter_of_ignores", 208, async |frame| {
        match rt::delegate(aignores(m)).await {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyRuntimeError>(py) => {
                let e = frame.catch(err);
                rt::builtins::print(py, ("  aouter caught", &e))?;
                return Ok(rt::str(py, "aouter-ret"));
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

// generator_close.py:216
pub fn run<'py>(
    m: &rt::Module<'py>,
    make: &Bound<'py, PyAny>,
    how: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "run", 216, |frame| {
        rt::builtins::print(py, (make.attr("__name__")?, how))?;
        let g = make.call0()?;
        rt::builtins::print(py, ("  first:", g.method1("send", (py.None(),))?))?;
        match (|| -> PyResult<()> {
            if how.eq("close")? {
                rt::builtins::print(py, ("  close ->", g.method0("close")?))?;
            } else if how.eq("throw GeneratorExit")? {
                rt::builtins::print(
                    py,
                    (
                        "  throw ->",
                        g.attr("throw")?
                            .call1((rt::builtin(py, "GeneratorExit")?,))?,
                    ),
                )?;
            } else {
                rt::builtins::print(
                    py,
                    (
                        "  throw ->",
                        g.attr("throw")?
                            .call1((rt::builtin(py, "ValueError")?.call1(("thrown",))?,))?,
                    ),
                )?;
            }
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyBaseException>(py) => {
                let e = frame.catch(err);
                rt::builtins::print(
                    py,
                    (
                        "  raised",
                        e.repr()?,
                        "context",
                        e.attr("__context__")?.repr()?,
                    ),
                )?;
            }
            Err(err) => return Err(err),
        }
        match (|| rt::builtins::print(py, ("  next ->", g.method1("send", (py.None(),))?)))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyBaseException>(py) => {
                let e = frame.catch(err);
                rt::builtins::print(py, ("  next raised", e.repr()?))?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

// generator_close.py:235
pub fn unraisable<'py>(m: &rt::Module<'py>, u: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "unraisable", 235, |_| {
        // CPython's message ends with the repr of the generator object, which a
        // delegate run in place does not have (design §12).
        rt::builtins::print(
            py,
            (
                "  unraisable:",
                u.attr("err_msg")?.method1("split", (" <",))?.get_item(0)?,
                u.attr("exc_value")?.repr()?,
            ),
        )?;
        Ok(rt::none(py))
    })
}

// generator_close.py:281
pub async fn delegates<'py>(
    m: &rt::Module<'py>,
    it: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "delegates", 281, async |_| { rt::yield_from(it).await })
}

// generator_close.py:285
pub async fn awaits<'py>(
    m: &rt::Module<'py>,
    it: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "awaits", 285, async |_| { rt::await_(it).await })
}

// Their classes look like CPython's.
// generator_close.py:304
pub async fn r#gen<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "gen", 304, async |_| {
        rt::yield_(rt::int(py, 1)).await?;
        Ok(rt::none(py))
    })
}

// generator_close.py:308
pub async fn coro<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "coro", 308, async |_| { Ok(rt::none(m.py())) })
}

pub mod sleep {
    use super::*;

    // generator_close.py:173
    /// The body of `class Sleep`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Sleep", 173, |_| {
            rt::def!(ns, gen __await__(slf as "self"))?;
            Ok(())
        })
    }

    // generator_close.py:174
    pub async fn __await__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Sleep.__await__", 174, async |_| {
            rt::yield_(rt::str(py, "sleep")).await?;
            Ok(rt::none(py))
        })
    }
}

pub mod throwable {
    use super::*;

    // generator_close.py:266
    /// The body of `class Throwable`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Throwable", 266, |_| {
            rt::def!(ns, __iter__(slf as "self"))?;
            rt::def!(ns, __next__(slf as "self"))?;
            rt::def!(ns, __await__(slf as "self"))?;
            rt::def!(ns, throw(slf as "self", *args))?;
            Ok(())
        })
    }

    // generator_close.py:267
    pub fn __iter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(slf.clone())
    }

    // generator_close.py:270
    pub fn __next__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::str(m.py(), "next"))
    }

    // generator_close.py:273
    pub fn __await__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(slf.clone())
    }

    // generator_close.py:276
    pub fn throw<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        args: &Bound<'py, PyTuple>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Throwable.throw", 276, |frame| {
            let list = PyList::empty(py);
            for a in rt::iter(args.as_any())? {
                let a = a?;
                list.append(if a.is_instance(&rt::builtin(py, "type")?)? {
                    a.clone()
                } else {
                    a.repr()?.into_any()
                })?;
            }
            rt::builtins::print(py, ("  throw got", list))?;
            Err(frame.raise(PyStopIteration::new_err("after throw")))
        })
    }
}

/// Executes the body of `generator_close.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "close() and throw() of generators and coroutines delegating to another one\n\
                (design §8): `yield from g(x)` and `await f(x)` between native functions run\n\
                the delegate in place (`rt::delegate`), through a variable they create an\n\
                object; both must follow CPython's `_gen_throw`.",
        )?;
        m.set_global("sys", rt::import(m, "sys")?)?;
        rt::def!(m, via(f))?;
        rt::def!(m, gen swallows())?;
        rt::def!(m, gen outer())?;
        rt::def!(m, gen outer_via())?;
        rt::def!(m, gen yields_again())?;
        rt::def!(m, gen outer_of_yields_again())?;
        rt::def!(m, gen outer_of_yields_again_via())?;
        rt::def!(m, gen keeps_yielding())?;
        rt::def!(m, gen outer_of_keeps_yielding())?;
        rt::def!(m, gen raises_in_finally())?;
        rt::def!(m, gen outer_of_raises())?;
        rt::def!(m, gen leaf())?;
        rt::def!(m, gen middle())?;
        rt::def!(m, gen top())?;
        rt::def!(m, gen countdown(n))?;
        rt::def!(m, gen catches_thrown())?;
        rt::def!(m, gen outer_of_catches())?;
        m.set_global("Sleep", rt::class!(m, "Sleep"(), sleep::body, line = 173)?)?;
        rt::def!(m, async aswallows())?;
        rt::def!(m, async aouter())?;
        rt::def!(m, async aouter_via())?;
        rt::def!(m, async aignores())?;
        rt::def!(m, async aouter_of_ignores())?;
        rt::def!(m, run(make, how))?;
        rt::def!(m, unraisable(u))?;
        let unraisable2 = m.global("unraisable")?;
        m.global("sys")?.set_attr("unraisablehook", unraisable2)?;
        for item in rt::iter(
            PyList::new(py, ["close", "throw GeneratorExit", "throw ValueError"])?.as_any(),
        )? {
            m.set_global("how", item?)?;
            // generator_close.py:251
            fn lambda<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
                rt::frame!(m, "<lambda>", 251, |_| {
                    m.global("countdown")?.call1((3,))
                })
            }
            for item2 in rt::iter(
                PyList::new(
                    py,
                    [
                        &m.global("outer")?,
                        &m.global("outer_via")?,
                        &m.global("outer_of_yields_again")?,
                        &m.global("outer_of_yields_again_via")?,
                        &m.global("outer_of_keeps_yielding")?,
                        &m.global("outer_of_raises")?,
                        &m.global("top")?,
                        &rt::function!(m, lambda(), qualname = "<lambda>")?,
                        &m.global("outer_of_catches")?,
                        &m.global("aouter")?,
                        &m.global("aouter_via")?,
                        &m.global("aouter_of_ignores")?,
                    ],
                )?
                .as_any(),
            )? {
                m.set_global("make", item2?)?;
                run(m, &m.global("make")?, &m.global("how")?)?;
            }
            match Err(frame.raise(PyLookupError::new_err("the caller's"))) {
                Ok(()) => unreachable!(),
                Err(err) if err.is_instance_of::<PyLookupError>(py) => {
                    let _caught = frame.catch(err);
                    run(m, &m.global("top")?, &m.global("how")?)?;
                }
                Err(err) => return Err(err),
            }
        }
        let __unraisablehook__ = m.global("sys")?.attr("__unraisablehook__")?;
        m.global("sys")?
            .set_attr("unraisablehook", __unraisablehook__)?;
        // throw() passes on its arguments to a delegate that is not a generator.
        m.set_global(
            "Throwable",
            rt::class!(m, "Throwable"(), throwable::body, line = 266)?,
        )?;
        rt::def!(m, gen delegates(it))?;
        rt::def!(m, async awaits(it))?;
        for item3 in
            rt::iter(PyList::new(py, [&m.global("delegates")?, &m.global("awaits")?])?.as_any())?
        {
            m.set_global("make", item3?)?;
            for item4 in rt::iter(
                PyList::new(
                    py,
                    [
                        PyTuple::new(py, [&rt::builtin(py, "ValueError")?])?.as_any(),
                        PyTuple::new(py, [&rt::builtin(py, "ValueError")?.call1(("instance",))?])?
                            .as_any(),
                        PyTuple::new(
                            py,
                            [&rt::builtin(py, "ValueError")?, &rt::str(py, "message")],
                        )?
                        .as_any(),
                    ],
                )?
                .as_any(),
            )? {
                m.set_global("args", item4?)?;
                m.set_global(
                    "g",
                    m.global("make")?
                        .call1((m.global("Throwable")?.call0()?,))?,
                )?;
                m.global("g")?.method1("send", (py.None(),))?;
                match (|| -> PyResult<()> {
                    m.set_global("warnings", rt::import(m, "warnings")?)?;
                    frame.with(&m.global("warnings")?.method0("catch_warnings")?, |_| {
                        m.global("warnings")?
                            .attr("simplefilter")?
                            .call1(("ignore", rt::builtin(py, "DeprecationWarning")?))?;
                        rt::call!(m.global("g")?.attr("throw")?, *m.global("args")?)?;
                        Ok(())
                    })?;
                    Ok(())
                })() {
                    Ok(()) => {}
                    Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
                        let exception = frame.catch(err);
                        m.set_global("e", &exception)?;
                        rt::builtins::print(
                            py,
                            (
                                m.global("make")?.attr("__name__")?,
                                "stopped with",
                                m.global("e")?.attr("value")?,
                            ),
                        )?;
                        m.unbind_global("e")?;
                    }
                    Err(err) => return Err(err),
                }
            }
        }
        rt::def!(m, gen r#gen())?;
        rt::def!(m, async coro())?;
        m.set_global("g", m.global("gen")?.call0()?)?;
        m.set_global("c", m.global("coro")?.call0()?)?;
        m.set_global("w", m.global("c")?.method0("__await__")?)?;
        // generator_close.py:315
        fn genexpr<'py>(
            m: &rt::Module<'py>,
            iter: &Bound<'py, PyIterator>,
        ) -> PyResult<Option<Bound<'py, PyAny>>> {
            rt::frame!(m, "<genexpr>", 315, |_| {
                if let Some(x) = rt::next(iter)? {
                    return Ok(Some(x));
                }
                Ok(None)
            })
        }
        for item5 in rt::iter(
            PyList::new(
                py,
                [
                    &m.global("g")?,
                    &m.global("c")?,
                    &m.global("w")?,
                    &rt::genexpr!(m, genexpr(PyList::empty(py).try_iter()?))?,
                ],
            )?
            .as_any(),
        )? {
            m.set_global("obj", item5?)?;
            rt::builtins::print(
                py,
                (
                    m.global("obj")?.get_type(),
                    m.global("obj")?.get_type().attr("__name__")?,
                    m.global("obj")?.get_type().attr("__module__")?,
                    m.global("obj")?.attr("__doc__")?.is_none(),
                ),
            )?;
            match (|| m.global("obj")?.get_item(0))() {
                Ok(_) => {}
                Err(err) if err.is_instance_of::<PyTypeError>(py) => {
                    let exception = frame.catch(err);
                    m.set_global("e", &exception)?;
                    rt::builtins::print(py, (m.global("e")?,))?;
                    m.unbind_global("e")?;
                }
                Err(err) => return Err(err),
            }
        }
        rt::builtins::print(py, (m.global("w")?.get_type().attr("__doc__")?,))?;
        m.global("c")?.method0("close")?;
        Ok(())
    })
}
