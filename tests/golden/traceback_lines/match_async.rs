use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::IntoPyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// match_async.py:8
#[rt::frame(m, "fetch", 8, frame)]
pub async fn fetch<'py>(
    m: &rt::Module<'py>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    frame.line(9);
    let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
    rt::await_(sleep).await?;
    Ok(value.clone())
}

// match_async.py:13
#[rt::frame(m, "allowed", 13, frame)]
pub async fn allowed<'py>(
    m: &rt::Module<'py>,
    name: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    frame.line(14);
    let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
    rt::await_(sleep).await?;
    frame.line(15);
    name.rich_compare("root", CompareOp::Ne)
}

// match_async.py:18
#[rt::frame(m, "handle", 18, frame)]
pub async fn handle<'py>(
    m: &rt::Module<'py>,
    request: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let extra: Bound<'py, PyAny>;
    let mut total: Bound<'py, PyAny>;
    {
        frame.line(19);
        let subject = rt::delegate(fetch(m, request)).await?;
        frame.line(20);
        if let Some([name, action2]) = rt::match_map(&subject, ["user", "action"])?
            && action2.eq("login")?
            && rt::delegate(allowed(m, &name)).await?.is_truthy()?
        {
            frame.line(21);
            let sleep = m.global("asyncio")?.method1("sleep", (0,))?;
            rt::await_(sleep).await?;
            frame.line(22);
            return rt::fstring(py, ("welcome ", rt::format(&name, "")?));
        } else {
            frame.line(23);
            if let Some([name, action2]) = rt::match_map(&subject, ["user", "action"])?
                && action2.eq("login")?
            {
                frame.line(24);
                return rt::fstring(py, ("denied ", rt::format(&name, "")?));
            } else {
                frame.line(25);
                if let Some([action]) = rt::match_map(&subject, ["action"])?
                    && let rest = rt::match_rest(&subject, ["action"])?
                {
                    frame.line(26);
                    extra = rt::delegate(fetch(m, &rt::builtins::sorted(py, (&rest,))?)).await?;
                    frame.line(27);
                    return rt::fstring(
                        py,
                        (rt::format(&action, "")?, " with ", rt::format(&extra, "")?),
                    );
                } else {
                    frame.line(28);
                    if let Some(([first], others, [])) = rt::match_seq_star(&subject)? {
                        total = first.clone();
                        frame.line(30);
                        for other in others.try_iter()? {
                            let other = other?;
                            frame.line(31);
                            total = rt::iadd(&total, rt::delegate(fetch(m, &other)).await?)?;
                            py.check_signals()?;
                            frame.line(30);
                        }
                        frame.line(32);
                        return rt::fstring(py, ("sum ", rt::format(&total, "")?));
                    }
                }
            }
        }
    }
    Ok(rt::str(py, "unknown"))
}

// match_async.py:36
#[rt::frame(m, "main", 36, frame)]
pub async fn main<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    frame.line(37);
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
    frame.line(44);
    for request in requests.try_iter()? {
        let request = request?;
        frame.line(45);
        rt::builtins::print(py, (rt::delegate(handle(m, &request)).await?,))?;
        py.check_signals()?;
        frame.line(44);
    }
    // A capture assigned before an `await` in the guard keeps its value.
    frame.line(47);
    let mut a: Option<Bound<'py, PyAny>> = None;
    let mut b: Option<Bound<'py, PyAny>> = None;
    'match_: {
        let subject = PyList::new(py, [5, 6])?.into_any();
        frame.line(48);
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
                frame.line(49);
                rt::builtins::print(py, ("no",))?;
                break 'match_;
            }
        }
        frame.line(51);
        rt::builtins::print(
            py,
            ("after the guard", rt::local(&a, "a")?, rt::local(&b, "b")?),
        )?;
    }
    Ok(rt::none(py))
}

// match_async.py:57
/// A generator: `yield` in the case bodies.
#[rt::frame(m, "tokens", 57, frame)]
pub async fn tokens<'py>(
    m: &rt::Module<'py>,
    text: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    frame.line(59);
    for word in text.method0("split")?.try_iter()? {
        let word = word?;
        frame.line(61);
        if word.eq("+")? || word.eq("-")? || word.eq("*")? || word.eq("/")? {
            let op = word.clone();
            frame.line(62);
            let value = PyTuple::new(py, [&rt::str(py, "op"), &op])?;
            rt::yield_(value).await?;
        } else {
            frame.line(63);
            if rt::match_instance(&word, &rt::builtin(py, "str")?)?
                && word.method0("isdigit")?.is_truthy()?
            {
                frame.line(64);
                let value =
                    PyTuple::new(py, [&rt::str(py, "num"), &rt::builtins::int(py, (&word,))?])?;
                rt::yield_(value).await?;
            } else {
                frame.line(65);
                if rt::match_instance(&word, &rt::builtin(py, "str")?)?
                    && word.method1("startswith", ("#",))?.is_truthy()?
                {
                    return Ok(rt::none(py));
                } else {
                    frame.line(68);
                    let value = PyTuple::new(py, [&rt::str(py, "name"), &word])?;
                    rt::yield_(value).await?;
                    frame.line(69);
                    let value = PyTuple::new(py, [&rt::str(py, "len"), &rt::int(py, word.len()?)])?;
                    rt::yield_(value).await?;
                }
            }
        }
        py.check_signals()?;
        frame.line(59);
    }
    Ok(rt::none(py))
}

// match_async.py:75
/// Captures used after a `yield` are kept in the generator's state.
#[rt::frame(m, "lines", 75, frame)]
pub async fn lines<'py>(
    m: &rt::Module<'py>,
    rows: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    frame.line(77);
    let mut received: Bound<'py, PyAny>;
    for row in rows.try_iter()? {
        let row = row?;
        frame.line(79);
        if let Some([key, value]) = rt::match_seq(&row)? {
            frame.line(80);
            received = rt::yield_(&key).await?;
            frame.line(81);
            rt::builtins::print(py, ("sent", &received, "for", &key, &value))?;
        }
        py.check_signals()?;
        frame.line(77);
    }
    frame.line(84);
    rt::yield_(rt::str(py, "done")).await?;
    Ok(rt::none(py))
}

/// Executes the body of `match_async.py` in the module `m`.
#[rt::frame(m, "<module>", 1, frame)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "`match` in native coroutines and generators (design §8): `await` in the\n\
            subject, a guard and a case body, `yield` in a body, captures kept across\n\
            suspensions.",
    )?;
    frame.line(5);
    m.set_global("asyncio", rt::import(m, "asyncio")?)?;
    rt::def!(m, async fetch(value))?;
    rt::def!(m, async allowed(name))?;
    rt::def!(m, async handle(request))?;
    rt::def!(m, async main())?;
    frame.line(54);
    m.global("asyncio")?
        .attr("run")?
        .call1((m.global("main")?.call0()?,))?;
    rt::def!(
        m,
        gen tokens(text),
        doc = "A generator: `yield` in the case bodies."
    )?;
    frame.line(72);
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
    frame.line(87);
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
    frame.line(88);
    rt::builtins::print(py, (rt::builtins::next(py, (m.global("gen")?,))?,))?;
    frame.line(89);
    rt::builtins::print(py, (m.global("gen")?.method1("send", ("x",))?,))?;
    frame.line(90);
    rt::builtins::print(py, (m.global("gen")?.method1("send", ("y",))?,))?;
    Ok(())
}
