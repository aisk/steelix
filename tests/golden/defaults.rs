use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// defaults.py:4
pub fn greet<'py>(
    m: &rt::Module<'py>,
    name: &Bound<'py, PyAny>,
    greeting: &Bound<'py, PyAny>,
    punct: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    greeting.add(", ")?.add(name)?.add(punct)
}

// defaults.py:8
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
        [start, stop] = rt::unpack(PyTuple::new(py, [&rt::int(py, 0), &start])?.as_any())?;
    }
    Ok(PyList::new(py, [&start, &stop, step])?.into_any())
}

// defaults.py:14
pub fn collect<'py>(
    m: &rt::Module<'py>,
    first: &Bound<'py, PyAny>,
    rest: &Bound<'py, PyTuple>,
    sep: &Bound<'py, PyAny>,
    options: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(PyList::new(m.py(), [first, rest.as_any(), sep, options.as_any()])?.into_any())
}

// defaults.py:18
pub fn append_to<'py>(
    m: &rt::Module<'py>,
    item: &Bound<'py, PyAny>,
    bucket: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    // The default list is created once and shared by every call.
    bucket.call_method1("append", (item,))?;
    Ok(bucket.clone())
}

// defaults.py:24
pub fn keyword_only<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    a.mul(10)?.add(b)
}

// defaults.py:28
pub fn scale<'py>(
    m: &rt::Module<'py>,
    x: &Bound<'py, PyAny>,
    factor: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    x.mul(factor)
}

/// Executes the body of `defaults.py` in the module `m`.
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
            m.global("collect")?.getattr("__defaults__")?,
            m.global("collect")?.getattr("__kwdefaults__")?,
            m.global("greet")?.getattr("__kwdefaults__")?,
        ),
    )?;
    // Defaults are read from the function object at call time.
    let value = PyTuple::new(py, ["Howdy"])?;
    m.global("greet")?.setattr("__defaults__", &value)?;
    let value2 = [("punct", "?!")].into_py_dict(py)?;
    m.global("greet")?.setattr("__kwdefaults__", &value2)?;
    let value3 = PyTuple::new(py, [3])?;
    m.global("scale")?.setattr("__defaults__", &value3)?;
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
    m.global("greet")?.setattr("__kwdefaults__", py.None())?;
    rt::builtins::print(
        py,
        (
            m.global("greet")?.getattr("__kwdefaults__")?,
            m.global("keyword_only")?.getattr("__defaults__")?,
        ),
    )?;
    rt::builtins::print(py, (m.global("append_to")?.getattr("__defaults__")?,))?;
    Ok(())
}
