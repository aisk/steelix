use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::IntoPyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// match_async.py:8
#[rt::frame(m, "fetch", 8)]
pub async fn fetch<'py>(
    m: &rt::Module<'py>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
    rt::await_(sleep).await?;
    Ok(value.clone())
}

// match_async.py:13
#[rt::frame(m, "allowed", 13)]
pub async fn allowed<'py>(
    m: &rt::Module<'py>,
    name: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
    rt::await_(sleep).await?;
    name.rich_compare("root", CompareOp::Ne)
}

// match_async.py:18
#[rt::frame(m, "handle", 18)]
pub async fn handle<'py>(
    m: &rt::Module<'py>,
    request: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let extra: Bound<'py, PyAny>;
    let mut total: Bound<'py, PyAny>;
    {
        let subject = rt::delegate(fetch(m, request)).await?;
        if let Some([name, action2]) = rt::match_map(&subject, ["user", "action"])?
            && action2.eq("login")?
            && rt::delegate(allowed(m, &name)).await?.is_truthy()?
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
            extra = rt::delegate(fetch(m, &rt::builtins::sorted(py, (&rest,))?)).await?;
            return rt::fstring(
                py,
                (rt::format(&action, "")?, " with ", rt::format(&extra, "")?),
            );
        } else if let Some(([first], others, [])) = rt::match_seq_star(&subject)? {
            total = first.clone();
            for other in rt::iter(&others)? {
                let other = other?;
                total = rt::iadd(&total, rt::delegate(fetch(m, &other)).await?)?;
            }
            return rt::fstring(py, ("sum ", rt::format(&total, "")?));
        }
    }
    Ok(rt::str(py, "unknown"))
}

// match_async.py:36
#[rt::frame(m, "main", 36)]
pub async fn main<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
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
    for request in rt::iter(&requests)? {
        let request = request?;
        rt::builtins::print(py, (rt::delegate(handle(m, &request)).await?,))?;
    }
    let mut a: Option<Bound<'py, PyAny>> = None;
    let mut b: Option<Bound<'py, PyAny>> = None;
    // A capture assigned before an `await` in the guard keeps its value.
    'match_: {
        let subject = PyList::new(py, [5, 6])?.into_any();
        if let Some([a2, b2]) = rt::match_seq(&subject)? {
            a = Some(a2);
            b = Some(b2);
            if rt::delegate(fetch(
                m,
                &rt::local(&a, "a")?.rich_compare(rt::local(&b, "b")?, CompareOp::Gt)?,
            ))
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
}

// match_async.py:57
/// A generator: `yield` in the case bodies.
#[rt::frame(m, "tokens", 57)]
pub async fn tokens<'py>(
    m: &rt::Module<'py>,
    text: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    for word in rt::iter(&text.method0("split")?)? {
        let word = word?;
        if word.eq("+")? || word.eq("-")? || word.eq("*")? || word.eq("/")? {
            let op = word.clone();
            let value = PyTuple::new(py, [&rt::str(py, "op"), &op])?;
            rt::yield_(value).await?;
        } else if rt::match_instance(&word, &rt::builtin(py, "str")?)?
            && word.method0("isdigit")?.is_truthy()?
        {
            let value = PyTuple::new(py, [&rt::str(py, "num"), &rt::builtins::int(py, (&word,))?])?;
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
    }
    Ok(rt::none(py))
}

// match_async.py:75
/// Captures used after a `yield` are kept in the generator's state.
#[rt::frame(m, "lines", 75)]
pub async fn lines<'py>(
    m: &rt::Module<'py>,
    rows: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut received: Bound<'py, PyAny>;
    for row in rt::iter(rows)? {
        let row = row?;
        if let Some([key, value]) = rt::match_seq(&row)? {
            received = rt::yield_(&key).await?;
            rt::builtins::print(py, ("sent", &received, "for", &key, &value))?;
        }
    }
    rt::yield_(rt::str(py, "done")).await?;
    Ok(rt::none(py))
}

/// Executes the body of `match_async.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
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
        (rt::builtins::list(
            py,
            (m.global("tokens")?
                .call1(("1 + x * 22 # rest is a comment",))?,),
        )?,),
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
    rt::builtins::print(py, (rt::builtins::next(py, (m.global("gen")?,))?,))?;
    rt::builtins::print(py, (m.global("gen")?.method1("send", ("x",))?,))?;
    rt::builtins::print(py, (m.global("gen")?.method1("send", ("y",))?,))?;
    Ok(())
}
