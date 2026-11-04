//! Parts of `coroutines.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 696, `def cancel_in_except`: except-as name used outside its handler

use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// --- native coroutines ---
// coroutines.py:28
pub async fn add_later<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "add_later", 28, async |_| {
        let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
        rt::await_(sleep).await?;
        a.add(b)
    })
}

// coroutines.py:33
pub async fn ticker<'py>(
    m: &rt::Module<'py>,
    name: &Bound<'py, PyAny>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "ticker", 33, async |_| {
        for i in m.global("range")?.call1((n,))?.try_iter()? {
            let i = i?;
            m.global("print")?.call1(("tick", name, &i))?;
            let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
            rt::await_(sleep).await?;
            m.py().check_signals()?;
        }
        name.mul(2)
    })
}

// coroutines.py:40
pub async fn combine<'py>(
    m: &rt::Module<'py>,
    x: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "combine", 40, async |_| {
        let awaitable = m.global("add_later")?.call1((x, 1))?;
        let direct = rt::await_(awaitable).await?;
        let alias = m.global("add_later")?;
        let through_object = rt::await_(alias.call1((x, 2))?).await?;
        let py_double = m.global("py_double")?.call1((x,))?;
        let from_python = rt::await_(py_double).await?;
        Ok(PyTuple::new(m.py(), [&direct, &through_object, &from_python])?.into_any())
    })
}

// coroutines.py:48
pub async fn guarded<'py>(
    m: &rt::Module<'py>,
    log: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "guarded", 48, async |frame| {
        let result: PyResult<()> = async {
            log.method1("append", ("started",))?;
            let sleep = m.global("asyncio")?.method1("sleep", (10,))?;
            rt::await_(sleep).await?;
            log.method1("append", ("not reached",))?;
            Ok(())
        }
        .await;
        let finally = frame.finally(result);
        log.method1("append", ("finally",))?;
        let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
        rt::await_(sleep).await?;
        log.attr("append")?
            .call1((rt::str(py, "after await in finally: ").add(
                m.global("type")?
                    .call1((m.global("sys")?.method0("exception")?,))?
                    .attr("__name__")?,
            )?,))?;
        finally.resume()?;
        Ok(rt::none(py))
    })
}

// coroutines.py:59
pub async fn failing<'py>(
    m: &rt::Module<'py>,
    x: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "failing", 59, async |frame| {
        let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
        rt::await_(sleep).await?;
        Err(frame.raise(
            m.global("ValueError")?
                .call1((rt::str(m.py(), "bad ").add(m.global("str")?.call1((x,))?)?,))?,
        ))
    })
}

// coroutines.py:64
pub async fn outer_fail<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "outer_fail", 64, async |_| {
        let awaitable = m.global("failing")?.call1((1,))?;
        rt::await_(awaitable).await?;
        Ok(rt::none(m.py()))
    })
}

// coroutines.py:68
pub async fn catcher<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "catcher", 68, async |frame| {
        match async {
            let awaitable = m.global("failing")?.call1((2,))?;
            rt::await_(awaitable).await
        }
        .await
        {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 68, || m.global("ValueError"))? => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                m.global("print")?.call1(("caught", &e))?;
                let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
                rt::await_(sleep).await?;
                m.global("print")?.call1((
                    "still handling",
                    m.global("repr")?
                        .call1((m.global("sys")?.method0("exception")?,))?,
                ))?;
            }
            Err(err) => return Err(err),
        }
        m.global("print")?.call1((
            "after handler",
            m.global("repr")?
                .call1((m.global("sys")?.method0("exception")?,))?,
        ))?;
        Ok(rt::none(m.py()))
    })
}

// coroutines.py:78
pub async fn echo<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "echo", 78, async |_| {
        let suspend = m.global("Suspend")?.call1(("first",))?;
        let a = rt::await_(suspend).await?;
        m.global("print")?.call1(("echo got", &a))?;
        let suspend = m.global("Suspend")?.call1(("second",))?;
        let b = rt::await_(suspend).await?;
        a.add(&b)
    })
}

// coroutines.py:85
pub async fn cleanup<'py>(
    m: &rt::Module<'py>,
    tag: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "cleanup", 85, async |frame| {
        let result: PyResult<()> = async {
            let suspend = m.global("Suspend")?.call1((tag,))?;
            rt::await_(suspend).await?;
            Ok(())
        }
        .await;
        let finally = frame.finally(result);
        m.global("print")?.call1(("cleanup finally", tag))?;
        finally.resume()?;
        Ok(rt::none(m.py()))
    })
}

// coroutines.py:92
pub async fn stubborn<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "stubborn", 92, async |frame| {
        match async {
            let suspend = m.global("Suspend")?.call1((1,))?;
            rt::await_(suspend).await
        }
        .await
        {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 92, || m.global("GeneratorExit"))? => {
                let _handling = rt::Handling::new(&frame.catch(err));
                m.global("print")?.call1(("ignoring GeneratorExit",))?;
                let awaitable = m.global("Suspend")?.call1((2,))?;
                rt::await_(awaitable).await?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(m.py()))
    })
}

// coroutines.py:100
pub async fn reenter<'py>(
    m: &rt::Module<'py>,
    r#box: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "reenter", 100, async |frame| {
        match (|| r#box.get_item(0)?.method1("send", (py.None(),)))() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 100, || m.global("ValueError"))? => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                m.global("print")?.call1(("reenter:", &e))?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::str(py, "reentered"))
    })
}

// coroutines.py:108
pub async fn stop_inside<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "stop_inside", 108, async |frame| {
        Err(frame.raise(m.global("StopIteration")?.call1((5,))?))
    })
}

// coroutines.py:112
pub async fn await_int<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "await_int", 112, async |_| {
        rt::await_(rt::int(py, 1)).await?;
        Ok(rt::none(py))
    })
}

// coroutines.py:116
pub async fn compute<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "compute", 116, async |_| {
        let request = m.global("Request")?.call1(("double", n))?;
        let doubled = rt::await_(request).await?;
        let request = m.global("Request")?.call1(("add", &doubled, 1))?;
        rt::await_(request).await
    })
}

// coroutines.py:121
pub async fn handler_state<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "handler_state", 121, async |frame| {
        match (|| -> PyResult<()> { Err(frame.raise(m.global("KeyError")?.call1((1,))?)) })() {
            Ok(()) => unreachable!(),
            Err(err) if frame.matches(&err, 121, || m.global("KeyError"))? => {
                let _handling = rt::Handling::new(&frame.catch(err));
                m.global("print")?.call1((
                    "in handler",
                    m.global("repr")?
                        .call1((m.global("sys")?.method0("exception")?,))?,
                ))?;
                let suspend = m.global("Suspend")?.call1(("in handler",))?;
                rt::await_(suspend).await?;
                m.global("print")?.call1((
                    "after resume",
                    m.global("repr")?
                        .call1((m.global("sys")?.method0("exception")?,))?,
                ))?;
            }
            Err(err) => return Err(err),
        }
        m.global("print")?.call1((
            "after the handler",
            m.global("repr")?
                .call1((m.global("sys")?.method0("exception")?,))?,
        ))?;
        Ok(rt::none(m.py()))
    })
}

// coroutines.py:131
pub async fn nested_guard<'py>(
    m: &rt::Module<'py>,
    log: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "nested_guard", 131, async |_| {
        let awaitable = m.global("guarded")?.call1((log,))?;
        rt::await_(awaitable).await?;
        Ok(rt::none(m.py()))
    })
}

// coroutines.py:135
pub async fn countdown<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "countdown", 135, async |_| {
        if n.eq(0)? {
            return Ok(rt::int(m.py(), 0));
        }
        let awaitable = m.global("countdown")?.call1((n.sub(1)?,))?;
        n.add(rt::await_(awaitable).await?)
    })
}

// --- native generators ---
// coroutines.py:144
pub async fn count_up<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "count_up", 144, async |_| {
        let mut i = rt::int(py, 0);
        while i.lt(n)? {
            rt::yield_(&i).await?;
            i = rt::iadd(&i, 1)?;
            py.check_signals()?;
        }
        Ok(rt::none(py))
    })
}

// coroutines.py:151
pub async fn accumulate<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "accumulate", 151, async |_| {
        let mut total = rt::int(py, 0);
        let mut x: Bound<'py, PyAny>;
        loop {
            x = rt::yield_(&total).await?;
            if x.is_none() {
                return Ok(total);
            }
            total = rt::iadd(&total, &x)?;
            py.check_signals()?;
        }
    })
}

// coroutines.py:160
pub async fn resilient<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "resilient", 160, async |frame| {
        loop {
            match rt::yield_(rt::str(py, "ready")).await {
                Ok(_) => {}
                Err(err) if frame.matches(&err, 160, || m.global("KeyError"))? => {
                    let e = frame.catch(err);
                    let _handling = rt::Handling::new(&e);
                    m.global("print")?
                        .call1(("handled", m.global("repr")?.call1((&e,))?))?;
                }
                Err(err) => return Err(err),
            }
            py.check_signals()?;
        }
    })
}

// coroutines.py:168
pub async fn with_finally<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "with_finally", 168, async |frame| {
        let result: PyResult<()> = async {
            rt::yield_(rt::int(py, 1)).await?;
            rt::yield_(rt::int(py, 2)).await?;
            Ok(())
        }
        .await;
        let finally = frame.finally(result);
        m.global("print")?.call1(("generator finally",))?;
        finally.resume()?;
        Ok(rt::none(py))
    })
}

// coroutines.py:176
pub async fn delegate<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "delegate", 176, async |_| {
        let py_inner = m.global("py_inner")?.call0()?;
        let result = rt::yield_from(py_inner).await?;
        m.global("print")?.call1(("delegate got", &result))?;
        rt::yield_(&result).await?;
        Ok(rt::none(m.py()))
    })
}

// coroutines.py:182
pub async fn in_with<'py>(
    m: &rt::Module<'py>,
    manager: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "in_with", 182, async |frame| {
        frame
            .with_async(manager, async |value| {
                rt::yield_(value).await?;
                rt::yield_(value.add(1)?).await?;
                Ok(())
            })
            .await?;
        Ok(rt::none(m.py()))
    })
}

// coroutines.py:188
pub async fn pep479<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "pep479", 188, async |_| {
        rt::yield_(rt::int(py, 1)).await?;
        m.global("next")?
            .call1((m.global("iter")?.call1((PyList::empty(py),))?,))?;
        Ok(rt::none(py))
    })
}

// coroutines.py:193
pub async fn bad_finally<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "bad_finally", 193, async |frame| {
        let result: PyResult<()> = async {
            rt::yield_(rt::int(m.py(), 1)).await?;
            Ok(())
        }
        .await;
        let _finally = frame.finally(result);
        Err(frame.raise(m.global("KeyError")?.call1(("in finally",))?))
    })
}

// coroutines.py:200
pub async fn closing_value<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "closing_value", 200, async |frame| {
        match rt::yield_(rt::int(py, 1)).await {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 200, || m.global("GeneratorExit"))? => {
                return Ok(rt::str(py, "closed value"));
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

// coroutines.py:207
pub async fn inner_items<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "inner_items", 207, async |frame| {
        let result: PyResult<()> = async {
            let mut received: Bound<'py, PyAny>;
            for i in m.global("range")?.call1((n,))?.try_iter()? {
                let i = i?;
                received = rt::yield_(&i).await?;
                if !received.is_none() {
                    m.global("print")?
                        .call1(("inner_items received", &received))?;
                }
                py.check_signals()?;
            }
            Ok(())
        }
        .await;
        let finally = frame.finally(result);
        m.global("print")?.call1(("inner_items finally",))?;
        finally.resume()?;
        Ok(rt::str(py, "inner_items done"))
    })
}

// coroutines.py:218
pub async fn outer_items<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "outer_items", 218, async |_| {
        let iterable = m.global("inner_items")?.call1((3,))?;
        let result = rt::yield_from(iterable).await?;
        m.global("print")?.call1(("outer_items got", &result))?;
        rt::yield_(&result).await?;
        Ok(rt::none(m.py()))
    })
}

// coroutines.py:224
pub async fn walk<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "walk", 224, async |_| {
        if n.is_truthy()? {
            let iterable = m.global("walk")?.call1((n.sub(1)?,))?;
            rt::yield_from(iterable).await?;
        }
        rt::yield_(n).await?;
        Ok(rt::none(m.py()))
    })
}

// coroutines.py:230
pub async fn chain<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "chain", 230, async |_| {
        if n.eq(0)? {
            return Ok(rt::int(py, 0));
        }
        let awaitable = m.global("chain")?.call1((n.sub(1)?,))?;
        rt::int(py, 1).add(rt::await_(awaitable).await?)
    })
}

// The same through generator and coroutine objects (not direct calls).
// coroutines.py:237
pub async fn walk_objects<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
    f: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "walk_objects", 237, async |_| {
        if n.is_truthy()? {
            let iterable = (if f.is_truthy()? {
                f.clone()
            } else {
                m.global("walk_objects")?
            })
            .call1((n.sub(1)?, f))?;
            rt::yield_from(iterable).await?;
        }
        rt::yield_(n).await?;
        Ok(rt::none(m.py()))
    })
}

// coroutines.py:243
pub async fn chain_objects<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
    f: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "chain_objects", 243, async |_| {
        if n.eq(0)? {
            return Ok(rt::int(py, 0));
        }
        let awaitable = (if f.is_truthy()? {
            f.clone()
        } else {
            m.global("chain_objects")?
        })
        .call1((n.sub(1)?, f))?;
        rt::int(py, 1).add(rt::await_(awaitable).await?)
    })
}

// coroutines.py:249
pub async fn in_except<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "in_except", 249, async |frame| {
        match (|| -> PyResult<()> {
            Err(frame.raise(m.global("KeyError")?.call1(("in_except",))?))
        })() {
            Ok(()) => unreachable!(),
            Err(err) if frame.matches(&err, 249, || m.global("KeyError"))? => {
                let _handling = rt::Handling::new(&frame.catch(err));
                rt::yield_(rt::str(py, "in except")).await?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

// coroutines.py:256
pub async fn sleep_in_except<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "sleep_in_except", 256, async |frame| {
        match (|| -> PyResult<()> {
            Err(frame.raise(m.global("KeyError")?.call1(("sleep_in_except",))?))
        })() {
            Ok(()) => unreachable!(),
            Err(err) if frame.matches(&err, 256, || m.global("KeyError"))? => {
                let _handling = rt::Handling::new(&frame.catch(err));
                let sleep = m.global("asyncio")?.method1("sleep", (10,))?;
                rt::await_(sleep).await?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(m.py()))
    })
}

// coroutines.py:263
pub async fn native_awaits<'py>(
    m: &rt::Module<'py>,
    awaitable: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "native_awaits", 263, async |_| {
        rt::await_(awaitable).await
    })
}

// coroutines.py:267
pub async fn owner_values<'py>(
    m: &rt::Module<'py>,
    slf: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "owner_values", 267, async |frame| {
        let result: PyResult<()> = async {
            for value in slf.attr("values")?.try_iter()? {
                let value = value?;
                rt::yield_(&value).await?;
                py.check_signals()?;
            }
            Ok(())
        }
        .await;
        let finally = frame.finally(result);
        m.global("print")?
            .call1(("owner_values finally", slf.attr("tag")?))?;
        finally.resume()?;
        Ok(rt::none(py))
    })
}

// coroutines.py:275
pub async fn owner_run<'py>(
    m: &rt::Module<'py>,
    slf: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "owner_run", 275, async |frame| {
        let result: PyResult<()> = async {
            let wait = slf.attr("event")?.method0("wait")?;
            rt::await_(wait).await?;
            Ok(())
        }
        .await;
        let finally = frame.finally(result);
        m.global("print")?
            .call1(("owner_run finally", slf.attr("tag")?))?;
        finally.resume()?;
        Ok(rt::none(m.py()))
    })
}

// coroutines.py:282
pub async fn recovered<'py>(
    m: &rt::Module<'py>,
    tag: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "recovered", 282, async |frame| {
        match (|| -> PyResult<()> { Err(frame.raise(m.global("KeyError")?.call1((tag,))?)) })() {
            Ok(()) => unreachable!(),
            Err(err) if frame.matches(&err, 282, || m.global("KeyError"))? => {}
            Err(err) => return Err(err),
        }
        let result: PyResult<()> = async {
            let suspend = m.global("Suspend")?.call1((tag,))?;
            rt::await_(suspend).await?;
            Ok(())
        }
        .await;
        let finally = frame.finally(result);
        m.global("print")?.call1(("recovered finally", tag))?;
        finally.resume()?;
        Ok(rt::none(m.py()))
    })
}

// coroutines.py:341
pub fn frames<'py>(m: &rt::Module<'py>, e: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "frames", 341, |_| {
        let list = PyList::empty(py);
        for f in m
            .global("traceback")?
            .attr("extract_tb")?
            .call1((e.attr("__traceback__")?,))?
            .try_iter()?
        {
            let f = f?;
            list.append(PyTuple::new(py, [&f.attr("name")?, &f.attr("lineno")?])?)?;
            py.check_signals()?;
        }
        Ok(list.into_any())
    })
}

// coroutines.py:345
pub fn unraisable<'py>(m: &rt::Module<'py>, u: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "unraisable", 345, |_| {
        let message = {
            let sub = m.global("re")?.attr("sub")?;
            let err_msg = u.attr("err_msg")?;
            sub.call1((
                " at 0x[0-9a-f]+",
                "",
                if err_msg.is_truthy()? {
                    err_msg
                } else {
                    rt::str(py, "")
                },
            ))?
        };
        m.global("print")?.call1((
            "unraisable:",
            u.attr("exc_type")?.attr("__name__")?,
            u.attr("exc_value")?,
            "|",
            &message,
        ))?;
        Ok(rt::none(py))
    })
}

// coroutines.py:353
pub async fn main<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "main", 353, async |frame| {
        {
            let print = m.global("print")?;
            let awaitable = m.global("combine")?.call1((10,))?;
            print.call1(("combine", rt::await_(awaitable).await?))?;
        }
        {
            let print = m.global("print")?;
            let gather = m.global("asyncio")?.attr("gather")?.call1((
                m.global("ticker")?.call1(("a", 2))?,
                m.global("ticker")?.call1(("b", 3))?,
            ))?;
            print.call1(("gather", rt::await_(gather).await?))?;
        }
        let mut log = PyList::empty(py).into_any();
        let mut task = m
            .global("asyncio")?
            .attr("create_task")?
            .call1((m.global("guarded")?.call1((&log,))?,))?;
        let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
        rt::await_(sleep).await?;
        task.method0("cancel")?;
        match rt::await_(&task).await {
            Ok(_) => {}
            Err(err)
                if frame.matches(&err, 353, || m.global("asyncio")?.attr("CancelledError"))? =>
            {
                let _handling = rt::Handling::new(&frame.catch(err));
                m.global("print")?
                    .call1(("cancelled", &log, task.method0("cancelled")?))?;
            }
            Err(err) => return Err(err),
        }
        // coroutines.py:364
        fn lambda<'py>(
            m: &rt::Module<'py>,
            log: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "main.<locals>.<lambda>", 364, |_| {
                m.global("py_awaits")?
                    .call1((m.global("guarded")?.call1((log,))?,))
            })
        }
        for make in PyTuple::new(
            py,
            [
                &m.global("nested_guard")?,
                &rt::function!(m, lambda(log), qualname = "main.<locals>.<lambda>")?,
            ],
        )?
        .try_iter()?
        {
            let make = make?;
            log = PyList::empty(py).into_any();
            task = m
                .global("asyncio")?
                .attr("create_task")?
                .call1((make.call1((&log,))?,))?;
            let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
            rt::await_(sleep).await?;
            task.method0("cancel")?;
            match rt::await_(&task).await {
                Ok(_) => {}
                Err(err)
                    if frame
                        .matches(&err, 353, || m.global("asyncio")?.attr("CancelledError"))? =>
                {
                    let e = frame.catch(err);
                    let _handling = rt::Handling::new(&e);
                    {
                        let print = m.global("print")?;
                        let list = PyList::empty(py);
                        for item in m.global("frames")?.call1((&e,))?.try_iter()? {
                            let [name, underscore] = rt::unpack(&item?)?;
                            list.append(&name)?;
                            py.check_signals()?;
                        }
                        print.call1(("cancelled", &log, list))?;
                    }
                }
                Err(err) => return Err(err),
            }
            py.check_signals()?;
        }
        match async {
            let awaitable = m.global("outer_fail")?.call0()?;
            rt::await_(awaitable).await
        }
        .await
        {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 353, || m.global("ValueError"))? => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                m.global("print")?
                    .call1(("traceback", &e, m.global("frames")?.call1((&e,))?))?;
            }
            Err(err) => return Err(err),
        }
        let awaitable2 = m.global("catcher")?.call0()?;
        rt::await_(awaitable2).await?;
        {
            let print = m.global("print")?;
            let awaitable2 = m.global("countdown")?.call1((30,))?;
            print.call1(("countdown", rt::await_(awaitable2).await?))?;
        }
        Ok(rt::none(py))
    })
}

// coroutines.py:544
pub fn trampoline<'py>(
    m: &rt::Module<'py>,
    coro: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "trampoline", 544, |frame| {
        let mut value = rt::none(py);
        let mut request: Bound<'py, PyAny>;
        let mut op: Bound<'py, PyAny>;
        let mut args: Bound<'py, PyAny>;
        loop {
            match coro.method1("send", (&value,)) {
                Ok(value2) => request = value2,
                Err(err) if frame.matches(&err, 544, || m.global("StopIteration"))? => {
                    let e = frame.catch(err);
                    let _handling = rt::Handling::new(&e);
                    return e.attr("value");
                }
                Err(err) => return Err(err),
            }
            ([op], args, []) = rt::unpack_star(&request.attr("args")?)?;
            m.global("print")?.call1(("request", &op, &args))?;
            value = if op.eq("double")? {
                args.get_item(0)?.mul(2)?
            } else {
                m.global("sum")?.call1((&args,))?
            };
            py.check_signals()?;
        }
    })
}

// A body suspended in an `except` block that is dropped (here it ignores
// `close()`) leaves the exception handled where it is dropped alone.
// coroutines.py:674
pub async fn stubborn_in_except<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "stubborn_in_except", 674, async |frame| {
        match (|| -> PyResult<()> { Err(frame.raise(m.global("KeyError")?.call1(("stubborn",))?)) })(
        ) {
            Ok(()) => unreachable!(),
            Err(err) if frame.matches(&err, 674, || m.global("KeyError"))? => {
                let _handling = rt::Handling::new(&frame.catch(err));
                loop {
                    match rt::yield_(rt::none(py)).await {
                        Ok(_) => {}
                        Err(err) if frame.matches(&err, 674, || m.global("GeneratorExit"))? => {}
                        Err(err) => return Err(err),
                    }
                    py.check_signals()?;
                }
            }
            Err(err) => Err(err),
        }
    })
}

// coroutines.py:715
pub async fn awaited_already<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "awaited_already", 715, async |frame| {
        // coroutines.py:716
        fn lambda<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "awaited_already.<locals>.<lambda>", 716, |_| {
                m.global("add_later")?.call1((1, 2))
            })
        }
        // coroutines.py:716
        fn lambda2<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "awaited_already.<locals>.<lambda>", 716, |_| {
                m.global("py_double")?.call1((1,))
            })
        }
        let mut c: Bound<'py, PyAny>;
        let mut task: Bound<'py, PyAny>;
        for make in PyTuple::new(
            py,
            [
                &rt::function!(m, lambda(), qualname = "awaited_already.<locals>.<lambda>")?,
                &rt::function!(m, lambda2(), qualname = "awaited_already.<locals>.<lambda>")?,
            ],
        )?
        .try_iter()?
        {
            let make = make?;
            c = make.call0()?;
            task = m.global("asyncio")?.method1("ensure_future", (&c,))?;
            let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
            rt::await_(sleep).await?;
            for waiter in PyTuple::new(py, [&m.global("py_awaits")?, &m.global("native_awaits")?])?
                .try_iter()?
            {
                let waiter = waiter?;
                match async { rt::await_(waiter.call1((&c,))?).await }.await {
                    Ok(_) => {}
                    Err(err) if frame.matches(&err, 715, || m.global("RuntimeError"))? => {
                        let e = frame.catch(err);
                        let _handling = rt::Handling::new(&e);
                        m.global("print")?.call1((
                            "awaited already:",
                            &e,
                            c.attr("cr_suspended")?,
                        ))?;
                    }
                    Err(err) => return Err(err),
                }
                py.check_signals()?;
            }
            m.global("print")?
                .call1(("task result", rt::await_(&task).await?))?;
            py.check_signals()?;
        }
        Ok(rt::none(py))
    })
}

// coroutines.py:753
pub fn report<'py>(
    m: &rt::Module<'py>,
    r#loop: &Bound<'py, PyAny>,
    context: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "report", 753, |_| {
        m.global("print")?
            .call1(("exception handler:", context.get_item("message")?))?;
        Ok(rt::none(m.py()))
    })
}

// coroutines.py:757
pub async fn owner_task<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "owner_task", 757, async |_| {
        m.global("asyncio")?
            .method0("get_running_loop")?
            .attr("set_exception_handler")?
            .call1((m.global("report")?,))?;
        let mut owner = Some(m.global("Owner")?.call1(("task",))?);
        let value = m.global("asyncio")?.method0("Event")?;
        rt::local(&owner, "owner")?.set_attr("event", value)?;
        let value = m
            .global("asyncio")?
            .attr("create_task")?
            .call1((rt::local(&owner, "owner")?.method0("run")?,))?;
        rt::local(&owner, "owner")?.set_attr("task", value)?;
        let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
        rt::await_(sleep).await?;
        owner = None;
        m.global("gc")?.method0("collect")?;
        m.global("print")?
            .call1(("after collecting an owner and its task",))?;
        Ok(rt::none(m.py()))
    })
}

/// Awaitable that hands its value to whoever drives the coroutine and
/// returns what is sent back.
pub mod suspend {
    use super::*;

    // coroutines.py:14
    /// The body of `class Suspend`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Suspend", 14, |_| {
            ns.set(
                "__doc__",
                "Awaitable that hands its value to whoever drives the coroutine and\n\
                    returns what is sent back.",
            )?;
            rt::def!(ns, __init__(slf as "self", value))?;
            rt::def!(ns, gen __await__(slf as "self"))?;
            Ok(())
        })
    }

    // coroutines.py:18
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        value: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Suspend.__init__", 18, |_| {
            slf.set_attr("value", value)?;
            Ok(rt::none(m.py()))
        })
    }

    // coroutines.py:21
    pub async fn __await__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Suspend.__await__", 21, async |_| {
            rt::yield_(slf.attr("value")?).await
        })
    }
}

pub mod manager {
    use super::*;

    // coroutines.py:323
    /// The body of `class Manager`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Manager", 323, |_| {
            rt::def!(ns, __enter__(slf as "self"))?;
            rt::def!(ns, __exit__(slf as "self", typ, value, tb))?;
            Ok(())
        })
    }

    // coroutines.py:324
    pub fn __enter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Manager.__enter__", 324, |_| {
            m.global("print")?.call1(("enter",))?;
            Ok(rt::int(m.py(), 10))
        })
    }

    // coroutines.py:328
    pub fn __exit__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        typ: &Bound<'py, PyAny>,
        value: &Bound<'py, PyAny>,
        tb: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Manager.__exit__", 328, |_| {
            m.global("print")?.call1((
                "exit",
                if typ.is_truthy()? {
                    typ.attr("__name__")?
                } else {
                    rt::none(py)
                },
            ))?;
            Ok(rt::bool(py, false))
        })
    }
}

pub mod request {
    use super::*;

    // coroutines.py:333
    /// The body of `class Request`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Request", 333, |_| {
            rt::def!(ns, __init__(slf as "self", *args))?;
            rt::def!(ns, gen __await__(slf as "self"))?;
            Ok(())
        })
    }

    // coroutines.py:334
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        args: &Bound<'py, PyTuple>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Request.__init__", 334, |_| {
            slf.set_attr("args", args)?;
            Ok(rt::none(m.py()))
        })
    }

    // coroutines.py:337
    pub async fn __await__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Request.__await__", 337, async |_| {
            rt::yield_(slf).await
        })
    }
}

pub mod holder {
    use super::*;

    // coroutines.py:506
    /// The body of `class Holder`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Holder", 506, |_| {
            rt::def!(ns, __str__(slf as "self"))?;
            Ok(())
        })
    }

    // coroutines.py:507
    pub fn __str__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(rt::str(m.py(), "holder"))
    }
}

pub mod owner {
    use super::*;

    // coroutines.py:733
    /// The body of `class Owner`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Owner", 733, |_| {
            ns.set("values_of", ns.get("owner_values")?)?;
            ns.set("run", ns.get("owner_run")?)?;
            rt::def!(ns, __init__(slf as "self", tag))?;
            rt::def!(ns, __del__(slf as "self"))?;
            Ok(())
        })
    }

    // coroutines.py:737
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        tag: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Owner.__init__", 737, |_| {
            slf.set_attr("tag", tag)?;
            slf.set_attr("values", PyList::new(py, [1, 2])?)?;
            Ok(rt::none(py))
        })
    }

    // coroutines.py:741
    pub fn __del__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Owner.__del__", 741, |_| {
            m.global("print")?.call1(("del owner", slf.attr("tag")?))?;
            Ok(rt::none(m.py()))
        })
    }
}

pub mod last {
    use super::*;

    // coroutines.py:775
    /// The body of `class Last`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Last", 775, |_| {
            rt::def!(ns, __del__(slf as "self"))?;
            Ok(())
        })
    }

    // coroutines.py:776
    pub fn __del__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Last.__del__", 776, |_| {
            m.global("print")?.call1(("del last",))?;
            Ok(rt::none(m.py()))
        })
    }
}

/// Executes the body of `coroutines.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "Native coroutines and generators (design §8): the program the M13 PoC\n\
                wrote by hand in Rust, driven by asyncio, by hand and by the GC.",
        )?;
        m.set_global("asyncio", rt::import(m, "asyncio")?)?;
        m.set_global("collections", rt::import(m, "collections.abc")?)?;
        m.set_global("gc", rt::import(m, "gc")?)?;
        m.set_global("inspect", rt::import(m, "inspect")?)?;
        m.set_global("re", rt::import(m, "re")?)?;
        m.set_global("sys", rt::import(m, "sys")?)?;
        m.set_global("traceback", rt::import(m, "traceback")?)?;
        m.set_global("warnings", rt::import(m, "warnings")?)?;
        m.set_global(
            "Suspend",
            rt::class!(
                m,
                "Suspend"(),
                suspend::body,
                line = 14,
                attributes = ["value"]
            )?,
        )?;
        rt::def!(m, async add_later(a, b))?;
        rt::def!(m, async ticker(name, n))?;
        rt::def!(m, async combine(x))?;
        rt::def!(m, async guarded(log))?;
        rt::def!(m, async failing(x))?;
        rt::def!(m, async outer_fail())?;
        rt::def!(m, async catcher())?;
        rt::def!(m, async echo())?;
        rt::def!(m, async cleanup(tag))?;
        rt::def!(m, async stubborn())?;
        rt::def!(m, async reenter(r#box))?;
        rt::def!(m, async stop_inside())?;
        rt::def!(m, async await_int())?;
        rt::def!(m, async compute(n))?;
        rt::def!(m, async handler_state())?;
        rt::def!(m, async nested_guard(log))?;
        rt::def!(m, async countdown(n))?;
        rt::def!(m, gen count_up(n))?;
        rt::def!(m, gen accumulate())?;
        rt::def!(m, gen resilient())?;
        rt::def!(m, gen with_finally())?;
        rt::def!(m, gen delegate())?;
        rt::def!(m, gen in_with(manager))?;
        rt::def!(m, gen pep479())?;
        rt::def!(m, gen bad_finally())?;
        rt::def!(m, gen closing_value())?;
        rt::def!(m, gen inner_items(n))?;
        rt::def!(m, gen outer_items())?;
        rt::def!(m, gen walk(n))?;
        rt::def!(m, async chain(n))?;
        rt::def!(m, gen walk_objects(n, f = py.None()))?;
        rt::def!(m, async chain_objects(n, f = py.None()))?;
        rt::def!(m, gen in_except())?;
        rt::def!(m, async sleep_in_except())?;
        rt::def!(m, async native_awaits(awaitable))?;
        rt::def!(m, gen owner_values(slf as "self"))?;
        rt::def!(m, async owner_run(slf as "self"))?;
        rt::def!(m, async recovered(tag))?;
        // --- CPython's own, mixed with the native ones ---
        m.global("exec")?.call1(("\nasync def py_awaits(awaitable):\n    return await awaitable\n\n\nasync def py_double(x):\n    await asyncio.sleep(0)\n    return x * 2\n\n\ndef py_inner():\n    try:\n        received = yield \"inner 1\"\n        print(\"inner received\", received)\n        try:\n            yield \"inner 2\"\n        except KeyError:\n            print(\"inner caught KeyError\")\n        yield \"inner 3\"\n    finally:\n        print(\"inner finally\")\n    return \"inner result\"\n", m.globals()))?;
        m.set_global(
            "Manager",
            rt::class!(m, "Manager"(), manager::body, line = 323)?,
        )?;
        m.set_global(
            "Request",
            rt::class!(
                m,
                "Request"(),
                request::body,
                line = 333,
                attributes = ["args"]
            )?,
        )?;
        rt::def!(m, frames(e))?;
        rt::def!(m, unraisable(u))?;
        let unraisable2 = m.global("unraisable")?;
        m.global("sys")?.set_attr("unraisablehook", unraisable2)?;
        rt::def!(m, async main())?;
        m.global("print")?.call1(("--- asyncio",))?;
        m.global("asyncio")?
            .attr("run")?
            .call1((m.global("main")?.call0()?,))?;
        m.global("print")?.call1(("--- send, throw, close",))?;
        m.set_global("c", m.global("echo")?.call0()?)?;
        m.global("print")?
            .call1((m.global("c")?.method1("send", (py.None(),))?,))?;
        m.global("print")?
            .call1((m.global("c")?.method1("send", (1,))?,))?;
        match (|| m.global("c")?.method1("send", (2,)))() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 1, || m.global("StopIteration"))? => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                let _handling = rt::Handling::new(&exception);
                m.global("print")?.call1((
                    "StopIteration",
                    m.global("e")?.attr("value")?,
                    m.global("e")?.attr("args")?,
                ))?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        match (|| m.global("c")?.method1("send", (py.None(),)))() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 1, || m.global("RuntimeError"))? => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                let _handling = rt::Handling::new(&exception);
                m.global("print")?.call1(("reuse:", m.global("e")?))?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        match (|| {
            m.global("c")?
                .attr("throw")?
                .call1((m.global("ValueError")?.call1(("late",))?,))
        })() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 1, || m.global("RuntimeError"))? => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                let _handling = rt::Handling::new(&exception);
                m.global("print")?.call1(("reuse throw:", m.global("e")?))?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        m.global("print")?
            .call1(("close after end", m.global("c")?.method0("close")?))?;
        m.set_global("c", m.global("echo")?.call0()?)?;
        m.global("c")?.method1("send", (py.None(),))?;
        match (|| {
            m.global("c")?
                .attr("throw")?
                .call1((m.global("KeyError")?.call1(("thrown",))?,))
        })() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 1, || m.global("KeyError"))? => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                let _handling = rt::Handling::new(&exception);
                m.global("print")?.call1((
                    "throw propagates",
                    m.global("repr")?.call1((m.global("e")?,))?,
                    m.global("frames")?
                        .call1((m.global("e")?,))?
                        .get_item(rt::slice(py, 1, py.None())?)?,
                ))?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        m.set_global("c", m.global("cleanup")?.call1(("closed",))?)?;
        m.global("print")?
            .call1((m.global("c")?.method1("send", (py.None(),))?,))?;
        m.global("print")?.call1((
            "close returns",
            m.global("c")?.method0("close")?,
            m.global("c")?.attr("cr_suspended")?,
        ))?;
        m.set_global("c", m.global("stubborn")?.call0()?)?;
        m.global("c")?.method1("send", (py.None(),))?;
        match (|| m.global("c")?.method0("close"))() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 1, || m.global("RuntimeError"))? => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                let _handling = rt::Handling::new(&exception);
                m.global("print")?.call1((m.global("e")?,))?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        m.global("print")?
            .call1(("still suspended", m.global("c")?.attr("cr_suspended")?))?;
        m.del_global("c")?;
        m.global("print")?.call1(("after del stubborn",))?;
        m.set_global("box", PyList::empty(py))?;
        m.set_global("c", m.global("reenter")?.call1((m.global("box")?,))?)?;
        m.global("box")?.attr("append")?.call1((m.global("c")?,))?;
        match (|| m.global("c")?.method1("send", (py.None(),)))() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 1, || m.global("StopIteration"))? => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                let _handling = rt::Handling::new(&exception);
                m.global("print")?
                    .call1(("reenter returned", m.global("e")?.attr("value")?))?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        m.del_global("box")?;
        m.del_global("c")?;
        match (|| {
            m.global("stop_inside")?
                .call0()?
                .method1("send", (py.None(),))
        })() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 1, || m.global("RuntimeError"))? => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                let _handling = rt::Handling::new(&exception);
                m.global("print")?.call1((
                    m.global("e")?,
                    m.global("repr")?
                        .call1((m.global("e")?.attr("__cause__")?,))?,
                ))?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        match (|| {
            m.global("await_int")?
                .call0()?
                .method1("send", (py.None(),))
        })() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 1, || m.global("TypeError"))? => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                let _handling = rt::Handling::new(&exception);
                m.global("print")?.call1(("TypeError:", m.global("e")?))?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        frame.with(
            &rt::call!(m.global("warnings")?.attr("catch_warnings")?, record = true)?,
            |value| {
                m.set_global("caught", value)?;
                m.global("warnings")?.method1("simplefilter", ("always",))?;
                match (|| m.global("echo")?.call0()?.method1("send", (1,)))() {
                    Ok(_) => {}
                    Err(err) if frame.matches(&err, 1, || m.global("TypeError"))? => {
                        let exception = frame.catch(err);
                        m.set_global("e", &exception)?;
                        let _handling = rt::Handling::new(&exception);
                        m.global("print")?.call1(("TypeError:", m.global("e")?))?;
                        m.unbind_global("e")?;
                    }
                    Err(err) => return Err(err),
                }
                m.global("gc")?.method0("collect")?;
                for item in m.global("caught")?.try_iter()? {
                    m.set_global("w", item?)?;
                    m.global("print")?.call1((
                        "warning",
                        m.global("w")?.attr("category")?.attr("__name__")?,
                        m.global("w")?.attr("message")?,
                    ))?;
                    py.check_signals()?;
                }
                Ok(())
            },
        )?;
        m.set_global("c", m.global("echo")?.call0()?)?;
        match (|| {
            m.global("c")?
                .attr("throw")?
                .call1((m.global("KeyError")?.call1(("before start",))?,))
        })() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 1, || m.global("KeyError"))? => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                let _handling = rt::Handling::new(&exception);
                m.global("print")?.call1((
                    "thrown before start",
                    m.global("repr")?.call1((m.global("e")?,))?,
                    m.global("frames")?
                        .call1((m.global("e")?,))?
                        .get_item(rt::slice(py, 1, py.None())?)?,
                ))?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        match (|| m.global("c")?.method1("send", (py.None(),)))() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 1, || m.global("RuntimeError"))? => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                let _handling = rt::Handling::new(&exception);
                m.global("print")?.call1(("after throw:", m.global("e")?))?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        m.set_global("c", m.global("handler_state")?.call0()?)?;
        m.global("print")?
            .call1((m.global("c")?.method1("send", (py.None(),))?,))?;
        m.global("print")?.call1((
            "outside",
            m.global("repr")?
                .call1((m.global("sys")?.method0("exception")?,))?,
        ))?;
        match (|| -> PyResult<()> { Err(frame.raise(m.global("ValueError")?.call1(("caller",))?)) })(
        ) {
            Ok(()) => unreachable!(),
            Err(err) if frame.matches(&err, 1, || m.global("ValueError"))? => {
                let _handling = rt::Handling::new(&frame.catch(err));
                match (|| m.global("c")?.method1("send", (py.None(),)))() {
                    Ok(_) => {}
                    Err(err) if frame.matches(&err, 1, || m.global("StopIteration"))? => {}
                    Err(err) => return Err(err),
                }
            }
            Err(err) => return Err(err),
        }
        m.global("print")?.call1(("--- finalization",))?;
        frame.with(
            &rt::call!(m.global("warnings")?.attr("catch_warnings")?, record = true)?,
            |value| {
                m.set_global("caught", value)?;
                m.global("warnings")?.method1("simplefilter", ("always",))?;
                m.set_global("c", m.global("add_later")?.call1((1, 2))?)?;
                m.del_global("c")?;
                m.global("gc")?.method0("collect")?;
                m.set_global("c", m.global("add_later")?.call1((1, 2))?)?;
                m.global("c")?.method0("close")?;
                m.del_global("c")?;
                for item2 in m.global("caught")?.try_iter()? {
                    m.set_global("w", item2?)?;
                    // The line is 0 for a warning without a Python frame (design §12).
                    m.global("print")?.call1((
                        "warning",
                        m.global("w")?.attr("category")?.attr("__name__")?,
                        m.global("w")?.attr("message")?,
                    ))?;
                    py.check_signals()?;
                }
                Ok(())
            },
        )?;
        frame.with(&m.global("warnings")?.method0("catch_warnings")?, |_| {
            m.global("warnings")?.method1("simplefilter", ("error",))?;
            m.set_global("c", m.global("add_later")?.call1((1, 2))?)?;
            m.del_global("c")?;
            m.global("print")?.call1(("warning as error reported",))?;
            Ok(())
        })?;
        m.set_global("c", m.global("cleanup")?.call1(("deleted",))?)?;
        m.global("c")?.method1("send", (py.None(),))?;
        m.global("print")?.call1(("before del",))?;
        m.del_global("c")?;
        m.global("print")?.call1(("after del",))?;
        m.set_global(
            "cycle",
            PyList::new(py, [&m.global("cleanup")?.call1(("in a cycle",))?])?,
        )?;
        m.global("cycle")?
            .attr("append")?
            .call1((m.global("cycle")?,))?;
        m.global("cycle")?
            .get_item(0)?
            .method1("send", (py.None(),))?;
        m.del_global("cycle")?;
        m.global("print")?.call1(("before collect",))?;
        m.global("gc")?.method0("collect")?;
        m.global("print")?.call1(("after collect",))?;
        m.set_global(
            "Holder",
            rt::class!(m, "Holder"(), holder::body, line = 506)?,
        )?;
        m.set_global("holder", m.global("Holder")?.call0()?)?;
        let value = m.global("cleanup")?.call1((m.global("holder")?,))?;
        m.global("holder")?.set_attr("coroutine", value)?;
        m.global("holder")?
            .attr("coroutine")?
            .method1("send", (py.None(),))?;
        m.del_global("holder")?;
        m.global("gc")?.method0("collect")?;
        m.global("print")?
            .call1(("after collecting a cycle through the locals",))?;
        m.set_global("w", m.global("with_finally")?.call0()?)?;
        m.global("next")?.call1((m.global("w")?,))?;
        m.global("print")?.call1(("before del generator",))?;
        m.del_global("w")?;
        m.global("print")?.call1(("after del generator",))?;
        m.set_global("b", m.global("bad_finally")?.call0()?)?;
        m.global("next")?.call1((m.global("b")?,))?;
        m.del_global("b")?;
        m.global("print")?.call1(("after bad_finally",))?;
        m.global("print")?.call1(("--- introspection",))?;
        m.set_global("c", m.global("add_later")?.call1((1, 2))?)?;
        m.global("print")?.call1((
            "iscoroutinefunction",
            m.global("inspect")?
                .attr("iscoroutinefunction")?
                .call1((m.global("add_later")?,))?,
        ))?;
        m.global("print")?.call1((
            "isgeneratorfunction",
            m.global("inspect")?
                .attr("isgeneratorfunction")?
                .call1((m.global("count_up")?,))?,
        ))?;
        m.global("print")?.call1((
            "asyncio.iscoroutine",
            m.global("asyncio")?
                .attr("iscoroutine")?
                .call1((m.global("c")?,))?,
        ))?;
        m.global("print")?.call1((
            "Coroutine abc",
            m.global("isinstance")?.call1((
                m.global("c")?,
                m.global("collections")?.attr("abc")?.attr("Coroutine")?,
            ))?,
        ))?;
        m.global("print")?.call1((
            "inspect.iscoroutine",
            m.global("inspect")?
                .attr("iscoroutine")?
                .call1((m.global("c")?,))?,
        ))?;
        m.global("print")?.call1((
            "names",
            m.global("c")?.attr("__name__")?,
            m.global("c")?.attr("__qualname__")?,
            m.global("repr")?
                .call1((m.global("c")?,))?
                .method1("split", (" at ",))?
                .get_item(0)?,
        ))?;
        m.global("c")?.method0("close")?;
        m.set_global("g", m.global("count_up")?.call1((1,))?)?;
        m.global("print")?.call1((
            "Generator abc",
            m.global("isinstance")?.call1((
                m.global("g")?,
                m.global("collections")?.attr("abc")?.attr("Generator")?,
            ))?,
        ))?;
        m.global("print")?.call1((
            "names",
            m.global("g")?.attr("__name__")?,
            m.global("g")?.attr("__qualname__")?,
            m.global("repr")?
                .call1((m.global("g")?,))?
                .method1("split", (" at ",))?
                .get_item(0)?,
        ))?;
        m.global("print")?.call1(("--- trampoline",))?;
        rt::def!(m, trampoline(coro))?;
        m.global("print")?.call1((
            "trampoline",
            m.global("trampoline")?
                .call1((m.global("compute")?.call1((20,))?,))?,
        ))?;
        m.global("print")?.call1(("--- generators",))?;
        m.global("print")?.call1((m
            .global("list")?
            .call1((m.global("count_up")?.call1((4,))?,))?,))?;
        for item3 in m.global("count_up")?.call1((2,))?.try_iter()? {
            m.set_global("i", item3?)?;
            m.global("print")?.call1(("for", m.global("i")?))?;
            py.check_signals()?;
        }
        m.set_global("acc", m.global("accumulate")?.call0()?)?;
        m.global("print")?.call1((
            m.global("next")?.call1((m.global("acc")?,))?,
            m.global("acc")?.method1("send", (5,))?,
            m.global("acc")?.method1("send", (7,))?,
        ))?;
        match (|| m.global("acc")?.method1("send", (py.None(),)))() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 1, || m.global("StopIteration"))? => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                let _handling = rt::Handling::new(&exception);
                m.global("print")?
                    .call1(("accumulated", m.global("e")?.attr("value")?))?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        m.set_global("r", m.global("resilient")?.call0()?)?;
        m.global("print")?
            .call1((m.global("next")?.call1((m.global("r")?,))?,))?;
        m.global("print")?.call1((m
            .global("r")?
            .attr("throw")?
            .call1((m.global("KeyError")?.call1(("k",))?,))?,))?;
        m.global("r")?.method0("close")?;
        m.global("print")?
            .call1(("closed", m.global("r")?.attr("gi_suspended")?))?;
        m.set_global("w", m.global("with_finally")?.call0()?)?;
        m.global("print")?
            .call1((m.global("next")?.call1((m.global("w")?,))?,))?;
        m.global("w")?.method0("close")?;
        m.global("print")?.call1(("closed with finally",))?;
        m.set_global("d", m.global("delegate")?.call0()?)?;
        m.global("print")?
            .call1((m.global("next")?.call1((m.global("d")?,))?,))?;
        m.global("print")?
            .call1((m.global("d")?.method1("send", ("hello",))?,))?;
        m.global("print")?.call1((m
            .global("d")?
            .attr("throw")?
            .call1((m.global("KeyError")?.call1(("x",))?,))?,))?;
        m.global("print")?
            .call1((m.global("next")?.call1((m.global("d")?,))?,))?;
        match (|| m.global("next")?.call1((m.global("d")?,)))() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 1, || m.global("StopIteration"))? => {
                let _handling = rt::Handling::new(&frame.catch(err));
                m.global("print")?.call1(("delegate done",))?;
            }
            Err(err) => return Err(err),
        }
        m.set_global("d", m.global("delegate")?.call0()?)?;
        m.global("next")?.call1((m.global("d")?,))?;
        m.global("d")?.method0("close")?;
        m.global("print")?.call1(("delegate closed",))?;
        m.set_global(
            "m",
            m.global("in_with")?
                .call1((m.global("Manager")?.call0()?,))?,
        )?;
        m.global("print")?.call1((
            m.global("next")?.call1((m.global("m")?,))?,
            m.global("next")?.call1((m.global("m")?,))?,
        ))?;
        m.global("m")?.method0("close")?;
        m.set_global(
            "m",
            m.global("in_with")?
                .call1((m.global("Manager")?.call0()?,))?,
        )?;
        m.global("print")?
            .call1((m.global("next")?.call1((m.global("m")?,))?,))?;
        match (|| {
            m.global("m")?
                .attr("throw")?
                .call1((m.global("ValueError")?.call1(("in with",))?,))
        })() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 1, || m.global("ValueError"))? => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                let _handling = rt::Handling::new(&exception);
                m.global("print")?.call1(("with saw", m.global("e")?))?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        m.set_global("v", m.global("closing_value")?.call0()?)?;
        m.global("next")?.call1((m.global("v")?,))?;
        m.global("print")?
            .call1(("close returns", m.global("v")?.method0("close")?))?;
        m.set_global("p", m.global("pep479")?.call0()?)?;
        m.global("print")?
            .call1((m.global("next")?.call1((m.global("p")?,))?,))?;
        match (|| m.global("next")?.call1((m.global("p")?,)))() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 1, || m.global("RuntimeError"))? => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                let _handling = rt::Handling::new(&exception);
                m.global("print")?.call1((
                    m.global("e")?,
                    m.global("repr")?
                        .call1((m.global("e")?.attr("__cause__")?,))?,
                ))?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        m.global("print")?
            .call1(("--- direct await of a native generator (yield from)",))?;
        m.set_global("o", m.global("outer_items")?.call0()?)?;
        m.global("print")?.call1((
            m.global("next")?.call1((m.global("o")?,))?,
            m.global("o")?.method1("send", ("sent",))?,
            m.global("next")?.call1((m.global("o")?,))?,
            m.global("next")?.call1((m.global("o")?,))?,
        ))?;
        match (|| m.global("next")?.call1((m.global("o")?,)))() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 1, || m.global("StopIteration"))? => {
                let _handling = rt::Handling::new(&frame.catch(err));
                m.global("print")?.call1(("outer_items done",))?;
            }
            Err(err) => return Err(err),
        }
        m.set_global("o", m.global("outer_items")?.call0()?)?;
        m.global("next")?.call1((m.global("o")?,))?;
        m.global("o")?.method0("close")?;
        m.global("print")?.call1(("outer_items closed",))?;
        m.set_global("o", m.global("outer_items")?.call0()?)?;
        m.global("next")?.call1((m.global("o")?,))?;
        match (|| {
            m.global("o")?
                .attr("throw")?
                .call1((m.global("KeyError")?.call1(("into inner",))?,))
        })() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 1, || m.global("KeyError"))? => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                let _handling = rt::Handling::new(&exception);
                {
                    let print = m.global("print")?;
                    let value = m.global("repr")?.call1((m.global("e")?,))?;
                    let list = PyList::empty(py);
                    for item4 in m.global("frames")?.call1((m.global("e")?,))?.try_iter()? {
                        let [name, underscore] = rt::unpack(&item4?)?;
                        list.append(&name)?;
                        py.check_signals()?;
                    }
                    print.call1(("thrown through", value, list))?;
                }
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        m.global("print")?.call1(("--- recursion depth",))?;
        // Too deep a chain raises `RecursionError`: CPython at the recursion limit,
        // native code when the C stack runs out (design §12), so only the type is
        // compared.
        for item5 in PyTuple::new(py, [100, 100000])?.try_iter()? {
            m.set_global("n", item5?)?;
            // coroutines.py:640
            fn lambda<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
                rt::frame!(m, "<lambda>", 640, |_| {
                    m.global("len")?.call1((m
                        .global("list")?
                        .call1((m.global("walk")?.call1((m.global("n")?,))?,))?,))
                })
            }
            // coroutines.py:641
            fn lambda2<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
                rt::frame!(m, "<lambda>", 641, |_| {
                    m.global("asyncio")?
                        .attr("run")?
                        .call1((m.global("chain")?.call1((m.global("n")?,))?,))
                })
            }
            // coroutines.py:642
            fn lambda3<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
                rt::frame!(m, "<lambda>", 642, |_| {
                    m.global("asyncio")?
                        .attr("run")?
                        .call1((m.global("countdown")?.call1((m.global("n")?,))?,))
                })
            }
            // coroutines.py:643
            fn lambda4<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
                rt::frame!(m, "<lambda>", 643, |_| {
                    m.global("len")?.call1((m
                        .global("list")?
                        .call1((m.global("walk_objects")?.call1((m.global("n")?,))?,))?,))
                })
            }
            // coroutines.py:644
            fn lambda5<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
                rt::frame!(m, "<lambda>", 644, |_| {
                    m.global("asyncio")?
                        .attr("run")?
                        .call1((m.global("chain_objects")?.call1((m.global("n")?,))?,))
                })
            }
            for item6 in PyTuple::new(
                py,
                [
                    PyTuple::new(
                        py,
                        [
                            &rt::str(py, "walk"),
                            &rt::function!(m, lambda(), qualname = "<lambda>")?,
                        ],
                    )?
                    .as_any(),
                    PyTuple::new(
                        py,
                        [
                            &rt::str(py, "chain"),
                            &rt::function!(m, lambda2(), qualname = "<lambda>")?,
                        ],
                    )?
                    .as_any(),
                    PyTuple::new(
                        py,
                        [
                            &rt::str(py, "countdown"),
                            &rt::function!(m, lambda3(), qualname = "<lambda>")?,
                        ],
                    )?
                    .as_any(),
                    PyTuple::new(
                        py,
                        [
                            &rt::str(py, "walk_objects"),
                            &rt::function!(m, lambda4(), qualname = "<lambda>")?,
                        ],
                    )?
                    .as_any(),
                    PyTuple::new(
                        py,
                        [
                            &rt::str(py, "chain_objects"),
                            &rt::function!(m, lambda5(), qualname = "<lambda>")?,
                        ],
                    )?
                    .as_any(),
                ],
            )?
            .try_iter()?
            {
                let [name, run] = rt::unpack(&item6?)?;
                m.set_global("name", name)?;
                m.set_global("run", run)?;
                match (|| {
                    m.global("print")?.call1((
                        m.global("name")?,
                        m.global("n")?,
                        m.global("run")?.call0()?,
                    ))
                })() {
                    Ok(_) => {}
                    Err(err) if frame.matches(&err, 1, || m.global("RecursionError"))? => {
                        let exception = frame.catch(err);
                        m.set_global("e", &exception)?;
                        let _handling = rt::Handling::new(&exception);
                        m.global("print")?.call1((
                            m.global("name")?,
                            m.global("n")?,
                            m.global("type")?
                                .call1((m.global("e")?,))?
                                .attr("__name__")?,
                        ))?;
                        m.unbind_global("e")?;
                    }
                    Err(err) => return Err(err),
                }
                py.check_signals()?;
            }
            py.check_signals()?;
        }
        m.global("print")?
            .call1(("--- context of thrown exceptions",))?;
        m.set_global("g", m.global("in_except")?.call0()?)?;
        m.global("print")?
            .call1((m.global("next")?.call1((m.global("g")?,))?,))?;
        match (|| {
            m.global("g")?
                .attr("throw")?
                .call1((m.global("ValueError")?.call1(("thrown",))?,))
        })() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 1, || m.global("ValueError"))? => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                let _handling = rt::Handling::new(&exception);
                m.global("print")?.call1((
                    "thrown context",
                    m.global("repr")?
                        .call1((m.global("e")?.attr("__context__")?,))?,
                ))?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        // A body that handles nothing itself gives what is thrown in no context,
        // whatever the caller handles.
        m.set_global("g", m.global("walk")?.call1((0,))?)?;
        m.global("next")?.call1((m.global("g")?,))?;
        match (|| -> PyResult<()> {
            Err(frame.raise(m.global("KeyError")?.call1(("the caller's",))?))
        })() {
            Ok(()) => unreachable!(),
            Err(err) if frame.matches(&err, 1, || m.global("KeyError"))? => {
                let _handling = rt::Handling::new(&frame.catch(err));
                match (|| {
                    m.global("g")?
                        .attr("throw")?
                        .call1((m.global("ValueError")?.call1(("thrown",))?,))
                })() {
                    Ok(_) => {}
                    Err(err) if frame.matches(&err, 1, || m.global("ValueError"))? => {
                        let exception = frame.catch(err);
                        m.set_global("e", &exception)?;
                        let _handling = rt::Handling::new(&exception);
                        m.global("print")?.call1((
                            "thrown context without a handler",
                            m.global("repr")?
                                .call1((m.global("e")?.attr("__context__")?,))?,
                        ))?;
                        m.unbind_global("e")?;
                    }
                    Err(err) => return Err(err),
                }
            }
            Err(err) => return Err(err),
        }
        rt::def!(m, gen stubborn_in_except())?;
        // coroutines.py:685
        fn lambda6<'py>(m: &rt::Module<'py>, u: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "<lambda>", 685, |_| {
                m.global("print")?.call1((
                    "unraisable:",
                    m.global("repr")?.call1((u.attr("exc_value")?,))?,
                ))
            })
        }
        let value = rt::function!(m, lambda6(u), qualname = "<lambda>")?;
        m.global("sys")?.set_attr("unraisablehook", value)?;
        m.set_global("g", m.global("stubborn_in_except")?.call0()?)?;
        m.global("next")?.call1((m.global("g")?,))?;
        match (|| -> PyResult<()> { Err(frame.raise(m.global("ValueError")?.call1(("mine",))?)) })()
        {
            Ok(()) => unreachable!(),
            Err(err) if frame.matches(&err, 1, || m.global("ValueError"))? => {
                let _handling = rt::Handling::new(&frame.catch(err));
                m.del_global("g")?;
                m.global("print")?.call1((
                    "handled after the drop",
                    m.global("repr")?
                        .call1((m.global("sys")?.method0("exception")?,))?,
                ))?;
            }
            Err(err) => return Err(err),
        }
        let __unraisablehook__ = m.global("sys")?.attr("__unraisablehook__")?;
        m.global("sys")?
            .set_attr("unraisablehook", __unraisablehook__)?;
        // coroutines.py:696, kept as Python: except-as name used outside its handler at 702:5
        rt::exec_python!(
            m,
            line = 696,
            r#"
async def cancel_in_except():
    task = asyncio.create_task(sleep_in_except())
    await asyncio.sleep(0)
    task.cancel()
    try:
        await task
    except asyncio.CancelledError as e:
        chain = []
        while e is not None:
            chain.append(repr(e))
            e = e.__context__
        print("cancelled chain", chain)
"#
        )?;
        m.global("asyncio")?
            .attr("run")?
            .call1((m.global("cancel_in_except")?.call0()?,))?;
        m.global("print")?.call1(("--- awaited already",))?;
        rt::def!(m, async awaited_already())?;
        m.global("asyncio")?
            .attr("run")?
            .call1((m.global("awaited_already")?.call0()?,))?;
        m.global("print")?.call1(("--- cycles through self",))?;
        m.set_global(
            "Owner",
            rt::class!(
                m,
                "Owner"(),
                owner::body,
                line = 733,
                attributes = ["tag", "values"]
            )?,
        )?;
        m.set_global("owner", m.global("Owner")?.call1(("generator",))?)?;
        let value = m.global("owner")?.method0("values_of")?;
        m.global("owner")?.set_attr("generator", value)?;
        m.global("print")?.call1((m
            .global("next")?
            .call1((m.global("owner")?.attr("generator")?,))?,))?;
        m.del_global("owner")?;
        m.global("gc")?.method0("collect")?;
        m.global("print")?
            .call1(("after collecting an owner and its generator",))?;
        rt::def!(m, report(r#loop, context))?;
        rt::def!(m, async owner_task())?;
        m.global("asyncio")?
            .attr("run")?
            .call1((m.global("owner_task")?.call0()?,))?;
        m.global("print")?.call1(("--- at exit",))?;
        // The hook is a function of this module: it would keep the globals alive.
        let __unraisablehook__ = m.global("sys")?.attr("__unraisablehook__")?;
        m.global("sys")?
            .set_attr("unraisablehook", __unraisablehook__)?;
        m.set_global("Last", rt::class!(m, "Last"(), last::body, line = 775)?)?;
        m.set_global("never_started", m.global("count_up")?.call1((3,))?)?;
        m.set_global("suspended_generator", m.global("with_finally")?.call0()?)?;
        m.global("next")?
            .call1((m.global("suspended_generator")?,))?;
        m.set_global(
            "suspended_coroutine",
            m.global("cleanup")?.call1(("at exit",))?,
        )?;
        m.global("suspended_coroutine")?
            .method1("send", (py.None(),))?;
        m.set_global(
            "after_an_error",
            m.global("recovered")?.call1(("after an error",))?,
        )?;
        m.global("after_an_error")?.method1("send", (py.None(),))?;
        m.set_global("last", m.global("Last")?.call0()?)?;
        m.global("print")?.call1(("end of script",))?;
        Ok(())
    })
}
