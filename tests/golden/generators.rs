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
#[rt::frame(m, "count", 7)]
pub async fn count<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut i = rt::int(py, 0);
    while i.lt(n)? {
        rt::yield_(&i).await?;
        i = rt::iadd(&i, 1)?;
        py.check_signals()?;
    }
    Ok(rt::none(py))
}

// generators.py:14
#[rt::frame(m, "averager", 14)]
pub async fn averager<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut total = rt::int(py, 0);
    let mut n = rt::int(py, 0);
    let mut average = rt::none(py);
    let mut value: Bound<'py, PyAny>;
    loop {
        value = rt::yield_(&average).await?;
        if value.is_none() {
            return Ok(total);
        }
        total = rt::iadd(&total, &value)?;
        n = rt::iadd(&n, 1)?;
        average = total.div(&n)?;
        py.check_signals()?;
    }
}

// generators.py:27
#[rt::frame(m, "bare", 27)]
pub async fn bare<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::yield_(rt::none(py)).await?;
    let x = rt::yield_(rt::none(py)).await?;
    rt::builtins::print(py, ("bare received", &x))?;
    Ok(rt::none(py))
}

// generators.py:33
#[rt::frame(m, "with_cleanup", 33, frame)]
pub async fn with_cleanup<'py>(
    m: &rt::Module<'py>,
    log: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let result: PyResult<()> = async {
        rt::yield_(rt::str(py, "first")).await?;
        rt::yield_(rt::str(py, "second")).await?;
        Ok(())
    }
    .await;
    let finally = frame.finally(result);
    log.method1("append", ("cleanup",))?;
    finally.resume()?;
    Ok(rt::none(py))
}

// generators.py:41
#[rt::frame(m, "catches", 41, frame)]
pub async fn catches<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    loop {
        match rt::yield_(rt::str(py, "ready")).await {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyValueError>(py) => {
                let e = frame.catch(err);
                rt::builtins::print(
                    py,
                    ("caught", &e, m.global("sys")?.method0("exception")?.repr()?),
                )?;
                rt::yield_(rt::str(py, "recovered")).await?;
            }
            Err(err) => return Err(err),
        }
        py.check_signals()?;
    }
}

// generators.py:50
#[rt::frame(m, "in_loops", 50)]
pub async fn in_loops<'py>(
    m: &rt::Module<'py>,
    rows: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    for row in rt::iter(rows)? {
        let row = row?;
        if row.is_none() {
            continue;
        }
        'for_else: {
            for item in rt::iter(&row)? {
                let item = item?;
                if item.eq("stop")? {
                    break 'for_else;
                }
                if item.eq("end")? {
                    return Ok(rt::str(py, "ended"));
                }
                rt::yield_(&item).await?;
            }
            rt::yield_(rt::str(py, "row done")).await?;
        }
    }
    Ok(rt::str(py, "all done"))
}

// generators.py:65
#[rt::frame(m, "in_with", 65, frame)]
pub async fn in_with<'py>(
    m: &rt::Module<'py>,
    manager: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    frame
        .with_async(manager, async |value| {
            rt::yield_(value).await?;
            rt::yield_(value.mul(2)?).await?;
            Ok(())
        })
        .await?;
    Ok(rt::none(m.py()))
}

// generators.py:84
#[rt::frame(m, "python_delegate", 84)]
pub async fn python_delegate<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let iter = PyList::new(py, [1, 2])?.try_iter()?;
    let mut result = rt::yield_from(iter).await?;
    rt::builtins::print(py, ("python delegate result", &result))?;
    result = rt::delegate(averager(m)).await?;
    Ok(result)
}

// generators.py:91
#[rt::frame(m, "native_delegate", 91)]
pub async fn native_delegate<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let total = rt::delegate(averager(m)).await?;
    rt::builtins::print(py, ("native delegate total", &total))?;
    let rest = rt::delegate(count(m, &rt::int(py, 2))).await?;
    rt::builtins::print(py, ("count returned", &rest))?;
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
}

// generators.py:100
/// Recursive `yield from` of the same native generator.
#[rt::frame(m, "tree", 100)]
pub async fn tree<'py>(
    m: &rt::Module<'py>,
    node: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    if node.is_instance(&rt::builtin(py, "list")?)? {
        for child in rt::iter(node)? {
            let child = child?;
            rt::delegate(Box::pin(tree(m, &child))).await?;
        }
    } else {
        rt::yield_(node).await?;
    }
    Ok(rt::none(py))
}

// generators.py:109
#[rt::frame(m, "ping", 109)]
pub async fn ping<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    if n.is_truthy()? {
        let value = PyTuple::new(py, [&rt::str(py, "ping"), n])?;
        rt::yield_(value).await?;
        rt::delegate(Box::pin(pong(m, &n.sub(1)?))).await?;
    }
    Ok(rt::none(py))
}

// generators.py:115
#[rt::frame(m, "pong", 115)]
pub async fn pong<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    if n.is_truthy()? {
        let value = PyTuple::new(py, [&rt::str(py, "pong"), n])?;
        rt::yield_(value).await?;
        rt::delegate(Box::pin(ping(m, &n.sub(1)?))).await?;
    }
    Ok(rt::none(py))
}

// generators.py:147
#[rt::frame(m, "tag", 147, frame)]
pub async fn tag<'py>(
    m: &rt::Module<'py>,
    name: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::builtins::print(py, (rt::fstring(py, ("<", rt::format(name, "")?, ">"))?,))?;
    let result: PyResult<()> = async {
        match async { rt::yield_(name.method0("upper")?).await }.await {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyZeroDivisionError>(py) => {
                let _caught = frame.catch(err);
                rt::builtins::print(py, ("contextmanager caught",))?;
            }
            Err(err) => return Err(err),
        }
        Ok(())
    }
    .await;
    let finally = frame.finally(result);
    rt::builtins::print(py, (rt::fstring(py, ("</", rt::format(name, "")?, ">"))?,))?;
    finally.resume()?;
    Ok(rt::none(py))
}

// generators.py:158
#[rt::frame(m, "make_counter", 158, frame)]
pub fn make_counter<'py>(
    m: &rt::Module<'py>,
    start: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let count = rt::Cell::new(m.py(), "count", start)?;
    // generators.py:161
    #[rt::frame(m, "make_counter.<locals>.counter", 161)]
    async fn counter<'py>(
        m: &rt::Module<'py>,
        count: &rt::Cell<'py>,
    ) -> PyResult<Bound<'py, PyAny>> {
        loop {
            count.set(rt::iadd(&count.get()?, 1)?)?;
            rt::yield_(count.get()?).await?;
            m.py().check_signals()?;
        }
    }
    let counter = rt::function!(frame, gen counter[&count]())?;
    counter.call0()
}

// generators.py:170
#[rt::frame(m, "closure_gen", 170, frame)]
pub fn closure_gen<'py>(
    m: &rt::Module<'py>,
    values: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let scale = rt::int(m.py(), 10);
    // generators.py:172
    #[rt::frame(m, "closure_gen.<locals>.<lambda>", 172)]
    fn lambda<'py>(
        m: &rt::Module<'py>,
        values: &Bound<'py, PyAny>,
        scale: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let list = PyList::empty(m.py());
        for v in rt::iter(values)? {
            let v = v?;
            list.append(v.mul(scale)?)?;
        }
        Ok(list.into_any())
    }
    rt::function!(frame, lambda as "<lambda>"[values, scale]())?.call0()
}

// generators.py:175
#[rt::frame(m, "gen_with_nested", 175, frame)]
pub async fn gen_with_nested<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    // generators.py:176
    #[rt::frame(m, "gen_with_nested.<locals>.inner", 176)]
    async fn inner<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::yield_(x).await?;
        rt::yield_(x.add(1)?).await?;
        Ok(rt::none(m.py()))
    }
    let inner = rt::function!(frame, gen inner(x))?;
    for value in rt::iter(&inner.call1((5,))?)? {
        let value = value?;
        rt::yield_(&value).await?;
    }
    Ok(rt::none(m.py()))
}

pub mod manager {
    use super::*;

    // generators.py:71
    /// The body of `class Manager`, run in its namespace `ns`.
    #[rt::frame(ns, "Manager", 71)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __init__(slf as "self", value))?;
        rt::def!(ns, __enter__(slf as "self"))?;
        rt::def!(ns, __exit__(slf as "self", typ, exc, tb))?;
        Ok(())
    }

    // generators.py:72
    #[rt::frame(m, "Manager.__init__", 72)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        value: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.set_attr("value", value)?;
        Ok(rt::none(m.py()))
    }

    // generators.py:75
    #[rt::frame(m, "Manager.__enter__", 75)]
    pub fn __enter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::builtins::print(m.py(), ("enter", slf.attr("value")?))?;
        slf.attr("value")
    }

    // generators.py:79
    #[rt::frame(m, "Manager.__exit__", 79)]
    pub fn __exit__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        typ: &Bound<'py, PyAny>,
        exc: &Bound<'py, PyAny>,
        tb: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
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
        Ok(rt::bool(py, typ.is(rt::builtin(py, "KeyError")?)))
    }
}

pub mod bag {
    use super::*;

    // generators.py:121
    /// The body of `class Bag`, run in its namespace `ns`.
    #[rt::frame(ns, "Bag", 121)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __init__(slf as "self", *items))?;
        rt::def!(ns, gen __iter__(slf as "self"))?;
        rt::def!(ns, gen pairs(slf as "self"))?;
        rt::def!(ns, @property doubled(slf as "self"))?;
        Ok(())
    }

    // generators.py:122
    #[rt::frame(m, "Bag.__init__", 122)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        items: &Bound<'py, PyTuple>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.set_attr("items", items)?;
        Ok(rt::none(m.py()))
    }

    // generators.py:125
    #[rt::frame(m, "Bag.__iter__", 125)]
    pub async fn __iter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        for item in rt::iter(&slf.attr("items")?)? {
            let item = item?;
            rt::yield_(&item).await?;
        }
        Ok(rt::none(m.py()))
    }

    // generators.py:129
    #[rt::frame(m, "Bag.pairs", 129)]
    pub async fn pairs<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        let mut previous = rt::none(py);
        for item in rt::iter(slf)? {
            let item = item?;
            if !previous.is_none() {
                rt::yield_(PyTuple::new(py, [&previous, &item])?).await?;
            }
            previous = item.clone();
        }
        Ok(rt::none(py))
    }

    // generators.py:136
    #[rt::frame(m, "Bag.doubled", 136, frame)]
    pub fn doubled<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // generators.py:138
        #[rt::frame(m, "Bag.doubled.<locals>.<genexpr>", 138)]
        fn genexpr<'py>(
            m: &rt::Module<'py>,
            iter: &Bound<'py, PyIterator>,
        ) -> PyResult<Option<Bound<'py, PyAny>>> {
            if let Some(x) = rt::next(iter)? {
                return Ok(Some(x.mul(2)?));
            }
            Ok(None)
        }
        rt::genexpr!(frame, genexpr(slf.try_iter()?))
    }
}

pub mod sub {
    use super::*;

    // generators.py:141
    /// The body of `class Sub`, run in its namespace `ns`.
    #[rt::frame(ns, "Sub", 141)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let __class__ = ns.class_cell();
        rt::def!(ns, gen __iter__[&__class__](slf as "self"))?;
        Ok(())
    }

    // generators.py:142
    #[rt::frame(m, "Sub.__iter__", 142)]
    pub async fn __iter__<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::yield_(rt::str(py, "sub")).await?;
        let __iter__ = rt::super_(__class__, slf)?.method0("__iter__")?;
        rt::yield_from(__iter__).await?;
        Ok(rt::none(py))
    }
}

/// Executes the body of `generators.py` in the module `m`.
#[rt::frame(m, "<module>", 1, frame)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "Generator functions lowered to native `async fn`s (design §8).",
    )?;
    m.set_global("contextlib", rt::import(m, "contextlib")?)?;
    m.set_global("sys", rt::import(m, "sys")?)?;
    rt::def!(m, gen count(n))?;
    rt::def!(m, gen averager())?;
    rt::def!(m, gen bare())?;
    rt::def!(m, gen with_cleanup(log))?;
    rt::def!(m, gen catches())?;
    rt::def!(m, gen in_loops(rows))?;
    rt::def!(m, gen in_with(manager))?;
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
    m.set_global(
        "Bag",
        rt::class!(m, "Bag"(), bag::body, line = 121, attributes = ["items"])?,
    )?;
    m.set_global(
        "Sub",
        rt::class!(m, "Sub"(m.global("Bag")?), sub::body, line = 141)?,
    )?;
    rt::def!(m, @contextlib.contextmanager gen tag(name))?;
    rt::def!(m, make_counter(start))?;
    rt::def!(m, closure_gen(values))?;
    rt::def!(m, gen gen_with_nested())?;
    rt::builtins::print(
        py,
        (rt::builtins::list(py, (m.global("count")?.call1((4,))?,))?,),
    )?;
    m.set_global("g", m.global("averager")?.call0()?)?;
    rt::builtins::print(
        py,
        (
            rt::builtins::next(py, (m.global("g")?,))?,
            m.global("g")?.method1("send", (10,))?,
            m.global("g")?.method1("send", (20,))?,
        ),
    )?;
    match (|| m.global("g")?.method1("send", (py.None(),)))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
            let exception = frame.catch(err);
            m.set_global("e", &exception)?;
            rt::builtins::print(py, ("returned", m.global("e")?.attr("value")?))?;
            m.unbind_global("e")?;
        }
        Err(err) => return Err(err),
    }
    m.set_global("b", m.global("bare")?.call0()?)?;
    rt::builtins::print(
        py,
        (
            rt::builtins::next(py, (m.global("b")?,))?,
            rt::builtins::next(py, (m.global("b")?,))?,
        ),
    )?;
    match (|| m.global("b")?.method1("send", ("hello",)))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
            let _caught = frame.catch(err);
            rt::builtins::print(py, ("bare done",))?;
        }
        Err(err) => return Err(err),
    }
    m.set_global("log", PyList::empty(py))?;
    m.set_global("c", m.global("with_cleanup")?.call1((m.global("log")?,))?)?;
    rt::builtins::print(py, (rt::builtins::next(py, (m.global("c")?,))?,))?;
    m.global("c")?.method0("close")?;
    rt::builtins::print(py, ("closed", m.global("log")?))?;
    m.set_global("c", m.global("with_cleanup")?.call1((m.global("log")?,))?)?;
    rt::builtins::print(
        py,
        (
            rt::builtins::next(py, (m.global("c")?,))?,
            rt::builtins::next(py, (m.global("c")?,))?,
        ),
    )?;
    match (|| rt::builtins::next(py, (m.global("c")?,)))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
            let _caught = frame.catch(err);
            rt::builtins::print(py, ("exhausted", m.global("log")?))?;
        }
        Err(err) => return Err(err),
    }
    m.set_global("r", m.global("catches")?.call0()?)?;
    rt::builtins::print(py, (rt::builtins::next(py, (m.global("r")?,))?,))?;
    rt::builtins::print(
        py,
        (m.global("r")?
            .attr("throw")?
            .call1((rt::builtin(py, "ValueError")?.call1(("boom",))?,))?,),
    )?;
    rt::builtins::print(py, (rt::builtins::next(py, (m.global("r")?,))?,))?;
    match (|| {
        m.global("r")?
            .attr("throw")?
            .call1((rt::builtin(py, "KeyError")?.call1(("other",))?,))
    })() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyKeyError>(py) => {
            let exception = frame.catch(err);
            m.set_global("e", &exception)?;
            rt::builtins::print(py, ("propagated", m.global("e")?.repr()?))?;
            m.unbind_global("e")?;
        }
        Err(err) => return Err(err),
    }
    rt::builtins::print(py, (m.global("sys")?.method0("exception")?.repr()?,))?;
    m.set_global(
        "loops",
        m.global("in_loops")?.call1((PyList::new(
            py,
            [
                PyList::new(py, [1, 2])?.as_any(),
                &rt::none(py),
                PyList::new(py, [&rt::int(py, 3), &rt::str(py, "stop"), &rt::int(py, 4)])?.as_any(),
                PyList::new(py, [5])?.as_any(),
            ],
        )?,))?,
    )?;
    rt::builtins::print(py, (rt::builtins::list(py, (m.global("loops")?,))?,))?;
    match (|| rt::builtins::next(py, (m.global("loops")?,)))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
            let exception = frame.catch(err);
            m.set_global("e", &exception)?;
            rt::builtins::print(py, ("loops stopped", m.global("e")?.attr("value")?))?;
            m.unbind_global("e")?;
        }
        Err(err) => return Err(err),
    }
    rt::builtins::print(
        py,
        (rt::builtins::list(
            py,
            (m.global("in_loops")?.call1((PyList::new(
                py,
                [
                    PyList::new(py, [&rt::int(py, 1), &rt::str(py, "end"), &rt::int(py, 2)])?
                        .as_any(),
                ],
            )?,))?,),
        )?,),
    )?;
    m.set_global(
        "w",
        m.global("in_with")?
            .call1((m.global("Manager")?.call1((3,))?,))?,
    )?;
    rt::builtins::print(py, (rt::builtins::list(py, (m.global("w")?,))?,))?;
    m.set_global(
        "w",
        m.global("in_with")?
            .call1((m.global("Manager")?.call1((4,))?,))?,
    )?;
    rt::builtins::print(py, (rt::builtins::next(py, (m.global("w")?,))?,))?;
    m.global("w")?.method0("close")?;
    m.set_global(
        "w",
        m.global("in_with")?
            .call1((m.global("Manager")?.call1((5,))?,))?,
    )?;
    rt::builtins::next(py, (m.global("w")?,))?;
    match (|| {
        m.global("w")?
            .attr("throw")?
            .call1((rt::builtin(py, "KeyError")?.call1(("suppressed",))?,))
    })() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
            let _caught = frame.catch(err);
            rt::builtins::print(py, ("suppressed by __exit__",))?;
        }
        Err(err) => return Err(err),
    }
    m.set_global("p", m.global("python_delegate")?.call0()?)?;
    rt::builtins::print(
        py,
        (
            rt::builtins::next(py, (m.global("p")?,))?,
            rt::builtins::next(py, (m.global("p")?,))?,
            rt::builtins::next(py, (m.global("p")?,))?,
            m.global("p")?.method1("send", (4,))?,
            m.global("p")?.method1("send", (6,))?,
        ),
    )?;
    match (|| m.global("p")?.method1("send", (py.None(),)))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
            let exception = frame.catch(err);
            m.set_global("e", &exception)?;
            rt::builtins::print(
                py,
                ("python_delegate returned", m.global("e")?.attr("value")?),
            )?;
            m.unbind_global("e")?;
        }
        Err(err) => return Err(err),
    }
    m.set_global("n", m.global("native_delegate")?.call0()?)?;
    rt::builtins::print(
        py,
        (
            rt::builtins::next(py, (m.global("n")?,))?,
            m.global("n")?.method1("send", (1,))?,
            m.global("n")?.method1("send", (3,))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("n")?.method1("send", (py.None(),))?,
            rt::builtins::next(py, (m.global("n")?,))?,
            rt::builtins::next(py, (m.global("n")?,))?,
        ),
    )?;
    match (|| rt::builtins::next(py, (m.global("n")?,)))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
            let exception = frame.catch(err);
            m.set_global("e", &exception)?;
            rt::builtins::print(
                py,
                ("native_delegate returned", m.global("e")?.attr("value")?),
            )?;
            m.unbind_global("e")?;
        }
        Err(err) => return Err(err),
    }
    rt::builtins::print(
        py,
        (rt::builtins::list(
            py,
            (m.global("tree")?.call1((PyList::new(
                py,
                [
                    &rt::int(py, 1),
                    PyList::new(py, [&rt::int(py, 2), PyList::new(py, [3, 4])?.as_any()])?.as_any(),
                    PyList::empty(py).as_any(),
                    &rt::int(py, 5),
                ],
            )?,))?,),
        )?,),
    )?;
    rt::builtins::print(
        py,
        (rt::builtins::list(py, (m.global("ping")?.call1((4,))?,))?,),
    )?;
    m.set_global("bag", m.global("Bag")?.call1((1, 2, 3))?)?;
    rt::builtins::print(
        py,
        (
            rt::builtins::list(py, (m.global("bag")?,))?,
            rt::builtins::list(py, (m.global("bag")?.method0("pairs")?,))?,
            rt::builtins::list(py, (m.global("bag")?.attr("doubled")?,))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (rt::builtins::list(py, (m.global("Sub")?.call1((7, 8))?,))?,),
    )?;
    frame.with(&m.global("tag")?.call1(("a",))?, |value| {
        m.set_global("name", value)?;
        rt::builtins::print(py, ("inside", m.global("name")?))?;
        Ok(())
    })?;
    frame.with(&m.global("tag")?.call1(("b",))?, |_| {
        rt::int(py, 1).div(0)?;
        Ok(())
    })?;
    rt::builtins::print(py, ("after b",))?;
    m.set_global("counter", make_counter(m, &rt::int(py, 5))?)?;
    rt::builtins::print(
        py,
        (
            rt::builtins::next(py, (m.global("counter")?,))?,
            rt::builtins::next(py, (m.global("counter")?,))?,
        ),
    )?;
    rt::builtins::print(py, (closure_gen(m, PyList::new(py, [1, 2])?.as_any())?,))?;
    rt::builtins::print(
        py,
        (rt::builtins::list(
            py,
            (m.global("gen_with_nested")?.call0()?,),
        )?,),
    )?;
    m.set_global("gen", m.global("count")?.call1((2,))?)?;
    rt::builtins::print(
        py,
        (
            m.global("gen")?.get_type().attr("__name__")?,
            m.global("gen")?.attr("__name__")?,
            m.global("gen")?.attr("__qualname__")?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            rt::builtins::next(py, (m.global("gen")?,))?,
            m.global("gen")?.attr("gi_running")?,
            m.global("gen")?.attr("gi_suspended")?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            rt::builtins::list(py, (m.global("gen")?,))?,
            m.global("gen")?.attr("gi_suspended")?,
        ),
    )?;
    Ok(())
}
