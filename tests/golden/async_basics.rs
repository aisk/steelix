use pyo3::exceptions::PyKeyError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// async_basics.py:8
#[rt::frame(m, "double", 8)]
pub async fn double<'py>(
    m: &rt::Module<'py>,
    x: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
    rt::await_(sleep).await?;
    x.mul(2)
}

// async_basics.py:13
/// Recursive direct awaits: `Box::pin`.
#[rt::frame(m, "fib", 13)]
pub async fn fib<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    if n.lt(2)? {
        return Ok(n.clone());
    }
    rt::delegate(Box::pin(fib(m, &n.sub(1)?)))
        .await?
        .add(rt::delegate(Box::pin(fib(m, &n.sub(2)?))).await?)
}

// async_basics.py:20
#[rt::frame(m, "is_even", 20)]
pub async fn is_even<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(if n.eq(0)? {
        rt::bool(m.py(), true)
    } else {
        rt::delegate(Box::pin(is_odd(m, &n.sub(1)?))).await?
    })
}

// async_basics.py:24
#[rt::frame(m, "is_odd", 24)]
pub async fn is_odd<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(if n.eq(0)? {
        rt::bool(m.py(), false)
    } else {
        rt::delegate(Box::pin(is_even(m, &n.sub(1)?))).await?
    })
}

// async_basics.py:28
#[rt::frame(m, "worker", 28)]
pub async fn worker<'py>(
    m: &rt::Module<'py>,
    name: &Bound<'py, PyAny>,
    delay: &Bound<'py, PyAny>,
    log: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    log.attr("append")?
        .call1((rt::fstring(py, (rt::format(name, "")?, " start"))?,))?;
    let sleep = m.global("asyncio")?.method1("sleep", (delay,))?;
    rt::await_(sleep).await?;
    log.attr("append")?
        .call1((rt::fstring(py, (rt::format(name, "")?, " end"))?,))?;
    Ok(name.clone())
}

// async_basics.py:35
#[rt::frame(m, "slow", 35, frame)]
pub async fn slow<'py>(
    m: &rt::Module<'py>,
    log: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let result: PyResult<()> = async {
        log.method1("append", ("slow start",))?;
        let sleep = m.global("asyncio")?.method1("sleep", (10,))?;
        rt::await_(sleep).await?;
        log.method1("append", ("slow end",))?;
        Ok(())
    }
    .await;
    let finally = frame.finally(result);
    log.method1("append", ("slow finally",))?;
    finally.resume()?;
    Ok(rt::none(m.py()))
}

// async_basics.py:44
#[rt::frame(m, "raises", 44, frame)]
pub async fn raises<'py>(
    m: &rt::Module<'py>,
    message: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
    rt::await_(sleep).await?;
    Err(frame.raise(PyValueError::new_err((message.clone().unbind(),))))
}

// async_basics.py:49
#[rt::frame(m, "handles", 49, frame)]
pub async fn handles<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    match rt::delegate(raises(m, &rt::str(py, "first"))).await {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyValueError>(py) => {
            let e = frame.catch(err);
            let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
            rt::await_(sleep).await?;
            rt::builtins::print(py, ("handled", &e))?;
        }
        Err(err) => return Err(err),
    }
    let value: Bound<'py, PyAny>;
    match rt::delegate(double(m, &rt::int(py, 21))).await {
        Ok(value2) => {
            value = value2;
            rt::builtins::print(py, ("else", &value))?;
        }
        Err(err) if err.is_instance_of::<PyKeyError>(py) => value = rt::none(py),
        Err(err) => return Err(err),
    }
    for i in rt::iter(&rt::builtins::range(py, (3,))?)? {
        let i = i?;
        let result: PyResult<rt::Flow<'py>> = async {
            if i.eq(1)? {
                return Ok(rt::Flow::Continue);
            }
            let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
            rt::await_(sleep).await?;
            if i.eq(2)? {
                return Ok(rt::Flow::Break);
            }
            Ok(rt::Flow::Next)
        }
        .await;
        let finally = frame.finally(result);
        rt::builtins::print(py, ("loop finally", &i))?;
        match finally.resume()? {
            rt::Flow::Break => break,
            rt::Flow::Continue => continue,
            _ => {}
        }
    }
    loop {
        match rt::delegate(raises(m, &rt::str(py, "in loop"))).await {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyValueError>(py) => {
                let _caught = frame.catch(err);
                return Ok(value);
            }
            Err(err) => return Err(err),
        }
        py.check_signals()?;
    }
}

// async_basics.py:77
#[rt::frame(m, "traced", 77, frame)]
pub fn traced<'py>(m: &rt::Module<'py>, r#fn: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    // async_basics.py:78
    #[rt::frame(m, "traced.<locals>.wrapper", 78)]
    async fn wrapper<'py>(
        m: &rt::Module<'py>,
        r#fn: &Bound<'py, PyAny>,
        args: &Bound<'py, PyTuple>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::builtins::print(py, ("call", r#fn.attr("__name__")?, args))?;
        let awaitable = rt::call!(r#fn, *args)?;
        let result = rt::await_(awaitable).await?;
        rt::builtins::print(py, ("result", &result))?;
        Ok(result)
    }
    let wrapper = rt::function!(
        frame,
        @(m.global("functools")?.method1("wraps", (r#fn,))?)
        async wrapper[r#fn](*args)
    )?;
    Ok(wrapper)
}

// async_basics.py:88
#[rt::frame(m, "add", 88)]
pub async fn add<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
    rt::await_(sleep).await?;
    a.add(b)
}

// async_basics.py:94
#[rt::frame(m, "make_accumulator", 94, frame)]
pub fn make_accumulator<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let total = rt::Cell::new(m.py(), "total", 0)?;
    // async_basics.py:97
    #[rt::frame(m, "make_accumulator.<locals>.add_to", 97)]
    async fn add_to<'py>(
        m: &rt::Module<'py>,
        total: &rt::Cell<'py>,
        x: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
        rt::await_(sleep).await?;
        total.set(rt::iadd(&total.get()?, x)?)?;
        total.get()
    }
    let add_to = rt::function!(frame, async add_to[&total](x))?;
    Ok(add_to)
}

// async_basics.py:106
#[rt::frame(m, "outer", 106, frame)]
pub async fn outer<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    // async_basics.py:107
    #[rt::frame(m, "outer.<locals>.inner", 107)]
    async fn inner<'py>(
        m: &rt::Module<'py>,
        n: &Bound<'py, PyAny>,
        k: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
        rt::await_(sleep).await?;
        n.mul(k)
    }
    let inner = rt::function!(frame, async inner[n](k))?;
    let results = PyList::empty(py);
    for k in rt::iter(&rt::builtins::range(py, (3,))?)? {
        let k = k?;
        results.append(rt::await_(inner.call1((&k,))?).await?)?;
    }
    Ok(results.into_any())
}

// async_basics.py:145
#[rt::frame(m, "main", 145, frame)]
pub async fn main<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::builtins::print(py, (rt::delegate(double(m, &rt::int(py, 4))).await?,))?;
    rt::builtins::print(py, (rt::delegate(fib(m, &rt::int(py, 10))).await?,))?;
    rt::builtins::print(
        py,
        (
            rt::delegate(is_even(m, &rt::int(py, 7))).await?,
            rt::delegate(is_odd(m, &rt::int(py, 7))).await?,
        ),
    )?;
    let mut log = PyList::empty(py);
    let gather = m.global("asyncio")?.attr("gather")?.call1((
        m.global("worker")?.call1(("a", 0.02, &log))?,
        m.global("worker")?.call1(("b", 0.01, &log))?,
        m.global("double")?.call1((5,))?,
    ))?;
    let results = rt::await_(gather).await?;
    rt::builtins::print(py, (&results, &log))?;
    log = PyList::empty(py);
    let task = m
        .global("asyncio")?
        .attr("create_task")?
        .call1((m.global("slow")?.call1((&log,))?,))?;
    let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
    rt::await_(sleep).await?;
    task.method0("cancel")?;
    match rt::await_(&task).await {
        Ok(_) => {}
        Err(err) if frame.matches(&err, || m.global("asyncio")?.attr("CancelledError"))? => {
            let _caught = frame.catch(err);
            rt::builtins::print(py, ("cancelled", &log, task.method0("cancelled")?))?;
        }
        Err(err) => return Err(err),
    }
    match async {
        let gather = m.global("asyncio")?.attr("gather")?.call1((
            m.global("raises")?.call1(("in gather",))?,
            m.global("double")?.call1((1,))?,
        ))?;
        rt::await_(gather).await
    }
    .await
    {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyValueError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("gather raised", &e))?;
        }
        Err(err) => return Err(err),
    }
    rt::builtins::print(py, ("handles returned", rt::delegate(handles(m)).await?))?;
    let awaitable = m.global("add")?.call1((1, 2))?;
    rt::builtins::print(py, (rt::await_(awaitable).await?,))?;
    let acc = make_accumulator(m)?;
    rt::builtins::print(
        py,
        (
            rt::await_(acc.call1((1,))?).await?,
            rt::await_(acc.call1((2,))?).await?,
        ),
    )?;
    rt::builtins::print(py, (rt::delegate(outer(m, &rt::int(py, 3))).await?,))?;
    let make = m.global("Derived")?.method1("make", (4,))?;
    let d = rt::await_(make).await?;
    let __name__ = d.get_type().attr("__name__")?;
    let value = rt::await_(d.method0("get")?).await?;
    let value2 = rt::await_(d.attr("value")?).await?;
    let awaitable = m.global("Base")?.method1("static", (1,))?;
    rt::builtins::print(py, (__name__, value, value2, rt::await_(awaitable).await?))?;
    // Through the function object: a coroutine object is created first.
    let coro = m.global("double")?.call1((8,))?;
    rt::builtins::print(
        py,
        (
            coro.get_type().attr("__name__")?,
            coro.attr("__name__")?,
            coro.attr("__qualname__")?,
            m.global("asyncio")?.method1("iscoroutine", (&coro,))?,
        ),
    )?;
    rt::builtins::print(py, (rt::await_(&coro).await?,))?;
    let alias = m.global("double")?;
    rt::builtins::print(py, (rt::await_(alias.call1((9,))?).await?,))?;
    rt::builtins::print(
        py,
        (
            m.global("inspect")?
                .attr("iscoroutinefunction")?
                .call1((m.global("double")?,))?,
            m.global("inspect")?
                .attr("iscoroutinefunction")?
                .call1((m.global("Derived")?.attr("get")?,))?,
        ),
    )?;
    let mut first: Option<Bound<'py, PyAny>> = None;
    let mut second: Option<Bound<'py, PyAny>> = None;
    frame
        .async_with(&m.global("asyncio")?.method0("TaskGroup")?, async |group| {
            first = Some(
                group
                    .attr("create_task")?
                    .call1((m.global("double")?.call1((1,))?,))?,
            );
            second = Some(
                group
                    .attr("create_task")?
                    .call1((m.global("double")?.call1((2,))?,))?,
            );
            Ok(())
        })
        .await?;
    rt::builtins::print(
        py,
        (
            rt::local(&first, "first")?.method0("result")?,
            rt::local(&second, "second")?.method0("result")?,
        ),
    )?;
    Ok(rt::none(py))
}

pub mod base {
    use super::*;

    // async_basics.py:117
    /// The body of `class Base`, run in its namespace `ns`.
    #[rt::frame(ns, "Base", 117)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __init__(slf as "self", value))?;
        rt::def!(ns, async get(slf as "self"))?;
        rt::def!(ns, @property value(slf as "self"))?;
        rt::def!(ns, @staticmethod async r#static(x))?;
        rt::def!(ns, @classmethod async make(cls, value))?;
        Ok(())
    }

    // async_basics.py:118
    #[rt::frame(m, "Base.__init__", 118)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        value: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        slf.set_attr("_value", value)?;
        Ok(())
    }

    // async_basics.py:121
    #[rt::frame(m, "Base.get", 121)]
    pub async fn get<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
        rt::await_(sleep).await?;
        slf.attr("_value")
    }

    // async_basics.py:125
    #[rt::frame(m, "Base.value", 125)]
    pub fn value<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        slf.method0("get")
    }

    // async_basics.py:129
    #[rt::frame(m, "Base.static", 129)]
    pub async fn r#static<'py>(
        m: &rt::Module<'py>,
        x: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        x.add(1)
    }

    // async_basics.py:133
    #[rt::frame(m, "Base.make", 133)]
    pub async fn make<'py>(
        m: &rt::Module<'py>,
        cls: &Bound<'py, PyAny>,
        value: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
        rt::await_(sleep).await?;
        cls.call1((value,))
    }
}

pub mod derived {
    use super::*;

    // async_basics.py:139
    /// The body of `class Derived`, run in its namespace `ns`.
    #[rt::frame(ns, "Derived", 139)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let __class__ = ns.class_cell();
        rt::def!(ns, async get[&__class__](slf as "self"))?;
        Ok(())
    }

    // async_basics.py:140
    #[rt::frame(m, "Derived.get", 140)]
    pub async fn get<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let get = rt::super_(__class__, slf)?.method0("get")?;
        let base = rt::await_(get).await?;
        base.mul(10)
    }
}

/// Executes the body of `async_basics.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    m.set_global(
        "__doc__",
        "`async def` lowered to native `async fn`s, run by asyncio (design §8).",
    )?;
    m.set_global("asyncio", rt::import(m, "asyncio")?)?;
    m.set_global("functools", rt::import(m, "functools")?)?;
    m.set_global("inspect", rt::import(m, "inspect")?)?;
    rt::def!(m, async double(x))?;
    rt::def!(
        m,
        async fib(n),
        doc = "Recursive direct awaits: `Box::pin`."
    )?;
    rt::def!(m, async is_even(n))?;
    rt::def!(m, async is_odd(n))?;
    rt::def!(m, async worker(name, delay, log))?;
    rt::def!(m, async slow(log))?;
    rt::def!(m, async raises(message))?;
    rt::def!(m, async handles())?;
    rt::def!(m, traced(r#fn))?;
    rt::def!(m, @traced async add(a, b))?;
    rt::def!(m, make_accumulator())?;
    rt::def!(m, async outer(n))?;
    m.set_global(
        "Base",
        rt::class!(m, "Base"(), base::body, line = 117, attributes = ["_value"])?,
    )?;
    m.set_global(
        "Derived",
        rt::class!(m, "Derived"(m.global("Base")?), derived::body, line = 139)?,
    )?;
    rt::def!(m, async main())?;
    m.global("asyncio")?
        .attr("run")?
        .call1((m.global("main")?.call0()?,))?;
    Ok(())
}
