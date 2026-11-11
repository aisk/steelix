use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// defaults.py:4
#[rt::frame(m, "greet", 4)]
pub fn greet<'py>(
    m: &rt::Module<'py>,
    name: &Bound<'py, PyAny>,
    greeting: &Bound<'py, PyAny>,
    punct: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    greeting.add(", ")?.add(name)?.add(punct)
}

// defaults.py:8
#[rt::frame(m, "span", 8)]
pub fn span<'py>(
    m: &rt::Module<'py>,
    start: &Bound<'py, PyAny>,
    stop: &Bound<'py, PyAny>,
    step: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut start = start.clone();
    let mut stop = stop.clone();
    if stop.is_none() {
        [start, stop] = [rt::int(py, 0), start.clone()];
    }
    Ok(PyList::new(py, [&start, &stop, step])?.into_any())
}

// defaults.py:14
#[rt::frame(m, "collect", 14)]
pub fn collect<'py>(
    m: &rt::Module<'py>,
    first: &Bound<'py, PyAny>,
    rest: &Bound<'py, PyTuple>,
    sep: &Bound<'py, PyAny>,
    options: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(PyList::new(m.py(), [first, rest, sep, options])?.into_any())
}

// defaults.py:18
#[rt::frame(m, "append_to", 18)]
pub fn append_to<'py>(
    m: &rt::Module<'py>,
    item: &Bound<'py, PyAny>,
    bucket: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    // The default list is created once and shared by every call.
    bucket.method1("append", (item,))?;
    Ok(bucket.clone())
}

// defaults.py:24
#[rt::frame(m, "keyword_only", 24)]
pub fn keyword_only<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    a.mul(10)?.add(b)
}

// defaults.py:28
#[rt::frame(m, "scale", 28)]
pub fn scale<'py>(
    m: &rt::Module<'py>,
    x: &Bound<'py, PyAny>,
    factor: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    x.mul(factor)
}

/// Executes the body of `defaults.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "Default, keyword, keyword-only, positional-only, *args and **kwargs parameters.",
    )?;
    rt::def!(m, greet(name, greeting = "Hello", *, punct = "!"))?;
    rt::def!(m, span(start, stop = py.None(), /, step = 1))?;
    rt::def!(m, collect(first, *rest, sep = "-", **options))?;
    rt::def!(m, append_to(item, bucket = PyList::empty(py)))?;
    rt::def!(m, keyword_only(*, a, b = 2))?;
    rt::def!(m, scale(x, factor = rt::int(py, 10).mul(2)?))?;
    // Direct calls with omitted defaults, keywords in any order, *args and **kwargs.
    rt::builtins::print(
        py,
        (
            greet(
                m,
                &rt::str(py, "Bob"),
                &m.default("greet", "greeting")?,
                &m.default("greet", "punct")?,
            )?,
            greet(
                m,
                &rt::str(py, "Al"),
                &rt::str(py, "Hi"),
                &m.default("greet", "punct")?,
            )?,
            greet(
                m,
                &rt::str(py, "X"),
                &m.default("greet", "greeting")?,
                &rt::str(py, "?"),
            )?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            span(
                m,
                &rt::int(py, 5),
                &m.default("span", "stop")?,
                &m.default("span", "step")?,
            )?,
            span(
                m,
                &rt::int(py, 2),
                &rt::int(py, 8),
                &m.default("span", "step")?,
            )?,
            span(m, &rt::int(py, 2), &rt::int(py, 8), &rt::int(py, 3))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            collect(
                m,
                &rt::int(py, 1),
                &PyTuple::empty(py),
                &m.default("collect", "sep")?,
                &PyDict::new(py),
            )?,
            collect(
                m,
                &rt::int(py, 1),
                &PyTuple::new(py, [2, 3])?,
                &rt::str(py, "+"),
                &[("x", 1), ("y", 2)].into_py_dict(py)?,
            )?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            append_to(m, &rt::int(py, 1), &m.default("append_to", "bucket")?)?,
            append_to(m, &rt::int(py, 2), &m.default("append_to", "bucket")?)?,
            append_to(m, &rt::int(py, 3), PyList::empty(py).as_any())?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            keyword_only(m, &rt::int(py, 1), &m.default("keyword_only", "b")?)?,
            keyword_only(m, &rt::int(py, 2), &rt::int(py, 5))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (scale(m, &rt::int(py, 2), &m.default("scale", "factor")?)?,),
    )?;
    // The same through the function objects.
    m.set_global("call", m.global("greet")?)?;
    rt::builtins::print(
        py,
        (
            m.global("call")?.call1(("Bob",))?,
            rt::call!(m.global("call")?, name = "Kim", punct = ".")?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("collect")?.attr("__defaults__")?,
            m.global("collect")?.attr("__kwdefaults__")?,
            m.global("greet")?.attr("__kwdefaults__")?,
        ),
    )?;
    // Defaults are read from the function object at call time.
    let value = PyTuple::new(py, ["Howdy"])?;
    m.global("greet")?.set_attr("__defaults__", value)?;
    let value = [("punct", "?!")].into_py_dict(py)?;
    m.global("greet")?.set_attr("__kwdefaults__", value)?;
    let value = PyTuple::new(py, [3])?;
    m.global("scale")?.set_attr("__defaults__", value)?;
    rt::builtins::print(
        py,
        (
            greet(
                m,
                &rt::str(py, "Sam"),
                &m.default("greet", "greeting")?,
                &m.default("greet", "punct")?,
            )?,
            m.global("call")?.call1(("Sam",))?,
            scale(m, &rt::int(py, 2), &m.default("scale", "factor")?)?,
        ),
    )?;
    m.global("greet")?.set_attr("__kwdefaults__", py.None())?;
    rt::builtins::print(
        py,
        (
            m.global("greet")?.attr("__kwdefaults__")?,
            m.global("keyword_only")?.attr("__defaults__")?,
        ),
    )?;
    rt::builtins::print(py, (m.global("append_to")?.attr("__defaults__")?,))?;
    Ok(())
}
