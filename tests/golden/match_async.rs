use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::IntoPyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// match_async.py:8
pub async fn fetch<'py>(
    m: &rt::Module<'py>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "fetch", 8, async |_| {
        let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
        rt::await_(sleep).await?;
        Ok(value.clone())
    })
}

// match_async.py:13
pub async fn allowed<'py>(
    m: &rt::Module<'py>,
    name: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "allowed", 13, async |_| {
        let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
        rt::await_(sleep).await?;
        name.rich_compare("root", CompareOp::Ne)
    })
}

// match_async.py:18
pub async fn handle<'py>(
    m: &rt::Module<'py>,
    request: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "handle", 18, async |_| {
        let extra: Bound<'py, PyAny>;
        let mut total: Bound<'py, PyAny>;
        {
            let subject = fetch(m, request).await?;
            if let Some([name, action2]) = rt::match_map(&subject, ["user", "action"])?
                && action2.eq("login")?
                && allowed(m, &name).await?.is_truthy()?
            {
                let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
                rt::await_(sleep).await?;
                return rt::fstring(py, ("welcome ", rt::format(&name, "")?));
            } else if let Some([name, action2]) = rt::match_map(&subject, ["user", "action"])?
                && action2.eq("login")?
            {
                return rt::fstring(py, ("denied ", rt::format(&name, "")?));
            } else if let Some([action]) = rt::match_map(&subject, ["action"])?
                && let rest = rt::match_rest(&subject, ["action"])?
            {
                extra = fetch(m, &rt::builtin(py, "sorted")?.call1((&rest,))?).await?;
                return rt::fstring(
                    py,
                    (rt::format(&action, "")?, " with ", rt::format(&extra, "")?),
                );
            } else if let Some(([first], others, [])) = rt::match_seq_star(&subject)? {
                total = first.clone();
                for other in others.try_iter()? {
                    let other = other?;
                    total = rt::iadd(&total, fetch(m, &other).await?)?;
                    py.check_signals()?;
                }
                return rt::fstring(py, ("sum ", rt::format(&total, "")?));
            }
        }
        Ok(rt::str(py, "unknown"))
    })
}

// match_async.py:36
pub async fn main<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "main", 36, async |_| {
        let requests = PyList::new(
            py,
            [
                [("user", "ann"), ("action", "login")]
                    .into_py_dict(py)?
                    .as_any(),
                [("user", "root"), ("action", "login")]
                    .into_py_dict(py)?
                    .as_any(),
                [
                    ("action", &rt::str(py, "logout")),
                    ("user", &rt::str(py, "ann")),
                    ("at", &rt::int(py, 3)),
                ]
                .into_py_dict(py)?
                .as_any(),
                PyList::new(py, [1, 2, 3])?.as_any(),
                &rt::str(py, "noise"),
            ],
        )?
        .into_any();
        for request in requests.try_iter()? {
            let request = request?;
            rt::builtins::print(py, (handle(m, &request).await?,))?;
            py.check_signals()?;
        }
        let mut a: Option<Bound<'py, PyAny>> = None;
        let mut b: Option<Bound<'py, PyAny>> = None;
        // A capture assigned before an `await` in the guard keeps its value.
        'match_: {
            let subject = PyList::new(py, [5, 6])?.into_any();
            if let Some([a2, b2]) = rt::match_seq(&subject)? {
                a = Some(a2);
                b = Some(b2);
                if fetch(
                    m,
                    &rt::local(&a, "a")?.rich_compare(rt::local(&b, "b")?, CompareOp::Gt)?,
                )
                .await?
                .is_truthy()?
                {
                    rt::builtins::print(py, ("no",))?;
                    break 'match_;
                }
            }
            rt::builtins::print(
                py,
                ("after the guard", rt::local(&a, "a")?, rt::local(&b, "b")?),
            )?;
        }
        Ok(rt::none(py))
    })
}

// match_async.py:57
/// A generator: `yield` in the case bodies.
pub async fn tokens<'py>(
    m: &rt::Module<'py>,
    text: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "tokens", 57, async |_| {
        for word in text.method0("split")?.try_iter()? {
            let word = word?;
            if word.eq("+")? || word.eq("-")? || word.eq("*")? || word.eq("/")? {
                let op = word.clone();
                let value = PyTuple::new(py, [&rt::str(py, "op"), &op])?;
                rt::yield_(value).await?;
            } else if rt::match_instance(&word, &rt::builtin(py, "str")?)?
                && word.method0("isdigit")?.is_truthy()?
            {
                let value = PyTuple::new(
                    py,
                    [
                        &rt::str(py, "num"),
                        &rt::builtin(py, "int")?.call1((&word,))?,
                    ],
                )?;
                rt::yield_(value).await?;
            } else if rt::match_instance(&word, &rt::builtin(py, "str")?)?
                && word.method1("startswith", ("#",))?.is_truthy()?
            {
                return Ok(rt::none(py));
            } else {
                let value = PyTuple::new(py, [&rt::str(py, "name"), &word])?;
                rt::yield_(value).await?;
                let value = PyTuple::new(py, [&rt::str(py, "len"), &rt::int(py, word.len()?)])?;
                rt::yield_(value).await?;
            }
            py.check_signals()?;
        }
        Ok(rt::none(py))
    })
}

// match_async.py:75
/// Captures used after a `yield` are kept in the generator's state.
pub async fn lines<'py>(
    m: &rt::Module<'py>,
    rows: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "lines", 75, async |_| {
        let mut received: Bound<'py, PyAny>;
        for row in rows.try_iter()? {
            let row = row?;
            if let Some([key, value]) = rt::match_seq(&row)? {
                received = rt::yield_(&key).await?;
                rt::builtins::print(py, ("sent", &received, "for", &key, &value))?;
            }
            py.check_signals()?;
        }
        rt::yield_(rt::str(py, "done")).await?;
        Ok(rt::none(py))
    })
}

/// Executes the body of `match_async.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global(
            "__doc__",
            "`match` in native coroutines and generators (design §8): `await` in the\n\
                subject, a guard and a case body, `yield` in a body, captures kept across\n\
                suspensions.",
        )?;
        m.set_global("asyncio", rt::import(m, "asyncio")?)?;
        rt::def!(m, async fetch(value))?;
        rt::def!(m, async allowed(name))?;
        rt::def!(m, async handle(request))?;
        rt::def!(m, async main())?;
        m.global("asyncio")?
            .attr("run")?
            .call1((m.global("main")?.call0()?,))?;
        rt::def!(
            m,
            gen tokens(text),
            doc = "A generator: `yield` in the case bodies."
        )?;
        rt::builtins::print(
            py,
            (rt::builtin(py, "list")?.call1((m
                .global("tokens")?
                .call1(("1 + x * 22 # rest is a comment",))?,))?,),
        )?;
        rt::def!(
            m,
            gen lines(rows),
            doc = "Captures used after a `yield` are kept in the generator's state."
        )?;
        m.set_global(
            "gen",
            m.global("lines")?.call1((PyList::new(
                py,
                [
                    PyTuple::new(py, [&rt::str(py, "a"), &rt::int(py, 1)])?.as_any(),
                    &rt::str(py, "skip"),
                    PyTuple::new(py, [&rt::str(py, "b"), &rt::int(py, 2)])?.as_any(),
                ],
            )?,))?,
        )?;
        rt::builtins::print(py, (rt::builtin(py, "next")?.call1((m.global("gen")?,))?,))?;
        rt::builtins::print(py, (m.global("gen")?.method1("send", ("x",))?,))?;
        rt::builtins::print(py, (m.global("gen")?.method1("send", ("y",))?,))?;
        Ok(())
    })
}
