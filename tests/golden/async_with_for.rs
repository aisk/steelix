use pyo3::exceptions::PyKeyError;
use pyo3::exceptions::PyStopAsyncIteration;
use pyo3::exceptions::PyTimeoutError;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// async_with_for.py:92
#[rt::frame(m, "with_flow", 92, frame)]
pub async fn with_flow<'py>(
    m: &rt::Module<'py>,
    items: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let found = PyList::empty(m.py()).into_any();
    for item in rt::iter(items)? {
        let item = item?;
        match frame
            .async_with(&m.global("Resource")?.call1((&item,))?, async |value| {
                if item.eq("skip")? {
                    return Ok(rt::Flow::Continue);
                }
                if item.eq("stop")? {
                    return Ok(rt::Flow::Break);
                }
                if item.eq("return")? {
                    return Ok(rt::Flow::Return(found.clone()));
                }
                found.method1("append", (value,))?;
                Ok(rt::Flow::Next)
            })
            .await?
        {
            rt::Flow::Return(value2) => return Ok(value2),
            rt::Flow::Break => break,
            rt::Flow::Continue => continue,
            rt::Flow::Next => {}
        }
    }
    Ok(found)
}

// async_with_for.py:106
#[rt::frame(m, "for_flow", 106)]
pub async fn for_flow<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let seen = PyList::empty(py).into_any();
    let mut i: Bound<'py, PyAny>;
    'for_else: {
        {
            let iter = rt::aiter(&m.global("Countdown")?.call1((n,))?)?;
            while let Some(item) = rt::anext(&iter).await? {
                i = item;
                if i.eq(3)? {
                    py.check_signals()?;
                    continue;
                }
                if i.eq(1)? {
                    break 'for_else;
                }
                seen.method1("append", (&i,))?;
                py.check_signals()?;
            }
        }
        seen.method1("append", ("no break",))?;
    }
    {
        let iter = rt::aiter(&m.global("Countdown")?.call1((2,))?)?;
        while let Some(item2) = rt::anext(&iter).await? {
            i = item2;
            seen.method1("append", (&i,))?;
            py.check_signals()?;
        }
    }
    seen.method1("append", ("else",))?;
    {
        let iter = rt::aiter(&m.global("SyncStop")?.call0()?)?;
        while let Some(underscore) = rt::anext(&iter).await? {
            seen.method1("append", ("never",))?;
            py.check_signals()?;
        }
    }
    Ok(seen)
}

// async_with_for.py:125
#[rt::frame(m, "nested", 125, frame)]
pub async fn nested<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    frame
        .async_with(&m.global("Resource")?.call1(("a",))?, async |a| {
            frame
                .async_with(&m.global("Resource")?.call1(("b",))?, async |b| {
                    rt::builtins::print(py, ("body", a, b))?;
                    Ok(())
                })
                .await
        })
        .await?;
    frame
        .async_with(
            &rt::call!(m.global("Resource")?, "c", suppress = true)?,
            async |_| -> PyResult<()> { Err(frame.raise(PyKeyError::new_err("suppressed"))) },
        )
        .await?;
    let result: PyResult<()> = async {
        frame
            .async_with(
                &m.global("Resource")?.call1(("d",))?,
                async |_| -> PyResult<()> { Err(frame.raise(PyKeyError::new_err("raised"))) },
            )
            .await?;
        Ok(())
    }
    .await;
    match result {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyKeyError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("propagated", e.repr()?))?;
        }
        Err(err) => return Err(err),
    }
    {
        let iter = rt::aiter(&m.global("Countdown")?.call1((2,))?)?;
        while let Some(i) = rt::anext(&iter).await? {
            frame
                .async_with(
                    &m.global("Resource")?
                        .call1((rt::fstring(py, ("loop ", rt::format(&i, "")?))?,))?,
                    async |value| {
                        rt::builtins::print(py, (value,))?;
                        Ok(())
                    },
                )
                .await?;
            py.check_signals()?;
        }
    }
    Ok(rt::none(py))
}

// async_with_for.py:140
#[rt::frame(m, "found_in", 140)]
pub async fn found_in<'py>(
    m: &rt::Module<'py>,
    source: &Bound<'py, PyAny>,
    wanted: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    {
        let iter = rt::aiter(source)?;
        while let Some(item) = rt::anext(&iter).await? {
            if item.eq(wanted)? {
                return Ok(rt::bool(py, true));
            }
            py.check_signals()?;
        }
    }
    Ok(rt::bool(py, false))
}

// async_with_for.py:147
#[rt::frame(m, "unpacking", 147)]
pub async fn unpacking<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let pairs = PyList::empty(py).into_any();
    {
        let iter = rt::aiter(&m.global("Pairs")?.call0()?)?;
        while let Some(item) = rt::anext(&iter).await? {
            let [a, b] = rt::unpack(&item)?;
            pairs.attr("append")?.call1((a.add(&b)?,))?;
            py.check_signals()?;
        }
    }
    Ok(pairs)
}

// async_with_for.py:167
#[rt::frame(m, "errors", 167, frame)]
pub async fn errors<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut cls: Bound<'py, PyAny>;
    for item in rt::iter(
        PyTuple::new(
            py,
            [
                &m.global("NoExit")?,
                &m.global("SyncOnly")?,
                &m.global("BadEnter")?,
                &m.global("BadExit")?,
                &rt::builtin(py, "int")?,
            ],
        )?
        .as_any(),
    )? {
        cls = item?;
        let result: PyResult<()> = async {
            frame
                .async_with(&cls.call0()?, async |_| {
                    rt::builtins::print(py, ("body of", cls.attr("__name__")?))?;
                    Ok(())
                })
                .await?;
            Ok(())
        }
        .await;
        match result {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyTypeError>(py) => {
                let e = frame.catch(err);
                rt::builtins::print(
                    py,
                    (
                        cls.attr("__name__")?,
                        &e,
                        e.attr("__cause__")?.repr()?,
                        e.attr("__context__")?.repr()?,
                    ),
                )?;
            }
            Err(err) => return Err(err),
        }
    }
    for item2 in rt::iter(
        PyTuple::new(
            py,
            [
                &m.global("BadNext")?,
                &m.global("BadIter")?,
                &m.global("NoIter")?,
            ],
        )?
        .as_any(),
    )? {
        cls = item2?;
        let result: PyResult<()> = async {
            {
                let iter = rt::aiter(&cls.call0()?)?;
                while let Some(x) = rt::anext(&iter).await? {
                    rt::builtins::print(py, ("item", &x))?;
                    py.check_signals()?;
                }
            }
            Ok(())
        }
        .await;
        match result {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyTypeError>(py) => {
                let e = frame.catch(err);
                rt::builtins::print(
                    py,
                    (
                        cls.attr("__name__")?,
                        &e,
                        e.attr("__cause__")?.repr()?,
                        e.attr("__context__")?.repr()?,
                    ),
                )?;
            }
            Err(err) => return Err(err),
        }
    }
    match (|| -> PyResult<()> {
        frame.with(&m.global("Resource")?.call1(("sync",))?, |_| Ok(()))?;
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, (&e,))?;
        }
        Err(err) => return Err(err),
    }
    Ok(rt::none(py))
}

// async_with_for.py:187
#[rt::frame(m, "library", 187, frame)]
pub async fn library<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let lock = m.global("asyncio")?.method0("Lock")?;
    frame
        .async_with(&lock, async |_| {
            rt::builtins::print(py, ("locked", lock.method0("locked")?))?;
            Ok(())
        })
        .await?;
    rt::builtins::print(py, ("unlocked", lock.method0("locked")?))?;
    let result: PyResult<()> = async {
        frame
            .async_with(
                &m.global("asyncio")?.method1("timeout", (0.01,))?,
                async |_| {
                    let sleep = m.global("asyncio")?.method1("sleep", (1,))?;
                    rt::await_(sleep).await?;
                    Ok(())
                },
            )
            .await?;
        Ok(())
    }
    .await;
    match result {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyTimeoutError>(py) => {
            let _caught = frame.catch(err);
            rt::builtins::print(py, ("timed out",))?;
        }
        Err(err) => return Err(err),
    }
    let queue = m.global("asyncio")?.method0("Queue")?;
    for i in rt::iter(&rt::builtin(py, "range")?.call1((3,))?)? {
        let i = i?;
        queue.method1("put_nowait", (&i,))?;
    }
    // async_with_for.py:201
    #[rt::frame(m, "library.<locals>.drain", 201)]
    async fn drain<'py>(
        m: &rt::Module<'py>,
        queue: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        let mut yield_value: Bound<'py, PyAny>;
        while !queue.method0("empty")?.is_truthy()? {
            yield_value = rt::await_(queue.method0("get")?).await?;
            rt::builtins::print(py, ("got", &yield_value))?;
            py.check_signals()?;
        }
        Ok(rt::none(py))
    }
    let drain = rt::function!(frame, async drain[queue]())?;
    rt::await_(drain.call0()?).await?;
    Ok(rt::none(py))
}

// async_with_for.py:209
#[rt::frame(m, "main", 209)]
pub async fn main<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::builtins::print(
        py,
        (rt::delegate(with_flow(
            m,
            PyList::new(py, ["x", "skip", "y", "stop", "z"])?.as_any(),
        ))
        .await?,),
    )?;
    rt::builtins::print(
        py,
        (rt::delegate(with_flow(m, PyList::new(py, ["x", "return"])?.as_any())).await?,),
    )?;
    rt::builtins::print(py, (rt::delegate(for_flow(m, &rt::int(py, 5))).await?,))?;
    rt::delegate(nested(m)).await?;
    rt::builtins::print(
        py,
        (
            rt::delegate(found_in(
                m,
                &m.global("Countdown")?.call1((5,))?,
                &rt::int(py, 2),
            ))
            .await?,
            rt::delegate(found_in(
                m,
                &m.global("Countdown")?.call1((2,))?,
                &rt::int(py, 7),
            ))
            .await?,
        ),
    )?;
    rt::builtins::print(py, (rt::delegate(unpacking(m)).await?,))?;
    rt::delegate(errors(m)).await?;
    rt::delegate(library(m)).await?;
    Ok(rt::none(py))
}

pub mod resource {
    use super::*;

    // async_with_for.py:7
    /// The body of `class Resource`, run in its namespace `ns`.
    #[rt::frame(ns, "Resource", 7)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __init__(slf as "self", name, suppress = false))?;
        rt::def!(ns, async __aenter__(slf as "self"))?;
        rt::def!(ns, async __aexit__(slf as "self", typ, exc, tb))?;
        Ok(())
    }

    // async_with_for.py:8
    #[rt::frame(m, "Resource.__init__", 8)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
        suppress: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.set_attr("name", name)?;
        slf.set_attr("suppress", suppress)?;
        Ok(rt::none(m.py()))
    }

    // async_with_for.py:12
    #[rt::frame(m, "Resource.__aenter__", 12)]
    pub async fn __aenter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::builtins::print(m.py(), ("enter", slf.attr("name")?))?;
        let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
        rt::await_(sleep).await?;
        slf.attr("name")?.method0("upper")
    }

    // async_with_for.py:17
    #[rt::frame(m, "Resource.__aexit__", 17)]
    pub async fn __aexit__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        typ: &Bound<'py, PyAny>,
        exc: &Bound<'py, PyAny>,
        tb: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
        rt::await_(sleep).await?;
        rt::builtins::print(
            py,
            (
                "exit",
                slf.attr("name")?,
                if typ.is_truthy()? {
                    typ.attr("__name__")?
                } else {
                    rt::none(py)
                },
            ),
        )?;
        slf.attr("suppress")
    }
}

pub mod countdown {
    use super::*;

    // async_with_for.py:23
    /// The body of `class Countdown`, run in its namespace `ns`.
    #[rt::frame(ns, "Countdown", 23)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __init__(slf as "self", n))?;
        rt::def!(ns, __aiter__(slf as "self"))?;
        rt::def!(ns, async __anext__(slf as "self"))?;
        Ok(())
    }

    // async_with_for.py:24
    #[rt::frame(m, "Countdown.__init__", 24)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        n: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.set_attr("n", n)?;
        Ok(rt::none(m.py()))
    }

    // async_with_for.py:27
    pub fn __aiter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(slf.clone())
    }

    // async_with_for.py:30
    #[rt::frame(m, "Countdown.__anext__", 30, frame)]
    pub async fn __anext__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        if slf.attr("n")?.le(0)? {
            return Err(frame.raise(PyStopAsyncIteration::new_err(())));
        }
        let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
        rt::await_(sleep).await?;
        slf.set_attr("n", rt::isub(&slf.attr("n")?, 1)?)?;
        slf.attr("n")
    }
}

pub mod sync_stop {
    use super::*;

    // async_with_for.py:38
    /// The body of `class SyncStop`, run in its namespace `ns`.
    #[rt::frame(ns, "SyncStop", 38)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __aiter__(slf as "self"))?;
        rt::def!(ns, __anext__(slf as "self"))?;
        Ok(())
    }

    // async_with_for.py:39
    pub fn __aiter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(slf.clone())
    }

    // async_with_for.py:42
    #[rt::frame(m, "SyncStop.__anext__", 42, frame)]
    pub fn __anext__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(frame.raise(PyStopAsyncIteration::new_err(())))
    }
}

pub mod no_exit {
    use super::*;

    // async_with_for.py:46
    /// The body of `class NoExit`, run in its namespace `ns`.
    #[rt::frame(ns, "NoExit", 46)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, async __aenter__(slf as "self"))?;
        Ok(())
    }

    // async_with_for.py:47
    #[rt::frame(m, "NoExit.__aenter__", 47)]
    pub async fn __aenter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(slf.clone())
    }
}

pub mod sync_only {
    use super::*;

    // async_with_for.py:51
    /// The body of `class SyncOnly`, run in its namespace `ns`.
    #[rt::frame(ns, "SyncOnly", 51)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __enter__(slf as "self"))?;
        rt::def!(ns, __exit__(slf as "self", *args))?;
        Ok(())
    }

    // async_with_for.py:52
    pub fn __enter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(slf.clone())
    }

    // async_with_for.py:55
    pub fn __exit__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        args: &Bound<'py, PyTuple>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::bool(m.py(), false))
    }
}

pub mod bad_enter {
    use super::*;

    // async_with_for.py:59
    /// The body of `class BadEnter`, run in its namespace `ns`.
    #[rt::frame(ns, "BadEnter", 59)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __aenter__(slf as "self"))?;
        rt::def!(ns, async __aexit__(slf as "self", *args))?;
        Ok(())
    }

    // async_with_for.py:60
    pub fn __aenter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::int(m.py(), 1))
    }

    // async_with_for.py:63
    #[rt::frame(m, "BadEnter.__aexit__", 63)]
    pub async fn __aexit__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        args: &Bound<'py, PyTuple>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::bool(m.py(), false))
    }
}

pub mod bad_exit {
    use super::*;

    // async_with_for.py:67
    /// The body of `class BadExit`, run in its namespace `ns`.
    #[rt::frame(ns, "BadExit", 67)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, async __aenter__(slf as "self"))?;
        rt::def!(ns, __aexit__(slf as "self", *args))?;
        Ok(())
    }

    // async_with_for.py:68
    #[rt::frame(m, "BadExit.__aenter__", 68)]
    pub async fn __aenter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(slf.clone())
    }

    // async_with_for.py:71
    pub fn __aexit__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        args: &Bound<'py, PyTuple>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::int(m.py(), 2))
    }
}

pub mod bad_next {
    use super::*;

    // async_with_for.py:75
    /// The body of `class BadNext`, run in its namespace `ns`.
    #[rt::frame(ns, "BadNext", 75)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __aiter__(slf as "self"))?;
        rt::def!(ns, __anext__(slf as "self"))?;
        Ok(())
    }

    // async_with_for.py:76
    pub fn __aiter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(slf.clone())
    }

    // async_with_for.py:79
    pub fn __anext__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::int(m.py(), 3))
    }
}

pub mod bad_iter {
    use super::*;

    // async_with_for.py:83
    /// The body of `class BadIter`, run in its namespace `ns`.
    #[rt::frame(ns, "BadIter", 83)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __aiter__(slf as "self"))?;
        Ok(())
    }

    // async_with_for.py:84
    pub fn __aiter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::int(m.py(), 4))
    }
}

pub mod no_iter {
    use super::*;

    // async_with_for.py:88
    /// The body of `class NoIter`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

pub mod pairs {
    use super::*;

    // async_with_for.py:154
    /// The body of `class Pairs`, run in its namespace `ns`.
    #[rt::frame(ns, "Pairs", 154)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __init__(slf as "self"))?;
        rt::def!(ns, __aiter__(slf as "self"))?;
        rt::def!(ns, async __anext__(slf as "self"))?;
        Ok(())
    }

    // async_with_for.py:155
    #[rt::frame(m, "Pairs.__init__", 155)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        slf.set_attr(
            "items",
            PyList::new(
                py,
                [
                    PyTuple::new(py, [1, 2])?.as_any(),
                    PyTuple::new(py, [3, 4])?.as_any(),
                ],
            )?,
        )?;
        Ok(rt::none(py))
    }

    // async_with_for.py:158
    pub fn __aiter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(slf.clone())
    }

    // async_with_for.py:161
    #[rt::frame(m, "Pairs.__anext__", 161, frame)]
    pub async fn __anext__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        if !slf.attr("items")?.is_truthy()? {
            return Err(frame.raise(PyStopAsyncIteration::new_err(())));
        }
        slf.attr("items")?.method1("pop", (0,))
    }
}

/// Executes the body of `async_with_for.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    m.set_global(
        "__doc__",
        "`async with` and `async for` (design §8): `frame.async_with`,\n\
            `rt::aiter` and `rt::anext`, with CPython's messages.",
    )?;
    m.set_global("asyncio", rt::import(m, "asyncio")?)?;
    m.set_global(
        "Resource",
        rt::class!(
            m,
            "Resource"(),
            resource::body,
            line = 7,
            attributes = ["name", "suppress"]
        )?,
    )?;
    m.set_global(
        "Countdown",
        rt::class!(
            m,
            "Countdown"(),
            countdown::body,
            line = 23,
            attributes = ["n"]
        )?,
    )?;
    m.set_global(
        "SyncStop",
        rt::class!(m, "SyncStop"(), sync_stop::body, line = 38)?,
    )?;
    m.set_global(
        "NoExit",
        rt::class!(m, "NoExit"(), no_exit::body, line = 46)?,
    )?;
    m.set_global(
        "SyncOnly",
        rt::class!(m, "SyncOnly"(), sync_only::body, line = 51)?,
    )?;
    m.set_global(
        "BadEnter",
        rt::class!(m, "BadEnter"(), bad_enter::body, line = 59)?,
    )?;
    m.set_global(
        "BadExit",
        rt::class!(m, "BadExit"(), bad_exit::body, line = 67)?,
    )?;
    m.set_global(
        "BadNext",
        rt::class!(m, "BadNext"(), bad_next::body, line = 75)?,
    )?;
    m.set_global(
        "BadIter",
        rt::class!(m, "BadIter"(), bad_iter::body, line = 83)?,
    )?;
    m.set_global(
        "NoIter",
        rt::class!(m, "NoIter"(), no_iter::body, line = 88)?,
    )?;
    rt::def!(m, async with_flow(items))?;
    rt::def!(m, async for_flow(n))?;
    rt::def!(m, async nested())?;
    rt::def!(m, async found_in(source, wanted))?;
    rt::def!(m, async unpacking())?;
    m.set_global(
        "Pairs",
        rt::class!(
            m,
            "Pairs"(),
            pairs::body,
            line = 154,
            attributes = ["items"]
        )?,
    )?;
    rt::def!(m, async errors())?;
    rt::def!(m, async library())?;
    rt::def!(m, async main())?;
    m.global("asyncio")?
        .attr("run")?
        .call1((m.global("main")?.call0()?,))?;
    Ok(())
}
