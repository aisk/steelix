use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// defaults.py:4
pub fn greet<'py>(
    py: Python<'py>,
    name: &Bound<'py, PyAny>,
    greeting: &Bound<'py, PyAny>,
    punct: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    greeting.add(", ")?.add(name)?.add(punct)
}

// defaults.py:8
pub fn span<'py>(
    py: Python<'py>,
    start: &Bound<'py, PyAny>,
    stop: &Bound<'py, PyAny>,
    step: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let mut start = start.clone();
    let mut stop = stop.clone();
    if stop.is_none() {
        [start, stop] = rt::unpack(PyTuple::new(py, [&rt::int(py, 0), &start])?.as_any())?;
    }
    Ok(PyList::new(py, [&start, &stop, step])?.into_any())
}

// defaults.py:14
pub fn collect<'py>(
    py: Python<'py>,
    first: &Bound<'py, PyAny>,
    rest: &Bound<'py, PyTuple>,
    sep: &Bound<'py, PyAny>,
    options: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(PyList::new(py, [first, rest.as_any(), sep, options.as_any()])?.into_any())
}

// defaults.py:18
pub fn append_to<'py>(
    py: Python<'py>,
    item: &Bound<'py, PyAny>,
    bucket: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    // The default list is created once and shared by every call.
    bucket.call_method1("append", (item,))?;
    Ok(bucket.clone())
}

// defaults.py:24
pub fn keyword_only<'py>(
    py: Python<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    a.mul(10)?.add(b)
}

// defaults.py:28
pub fn scale<'py>(
    py: Python<'py>,
    x: &Bound<'py, PyAny>,
    factor: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    x.mul(factor)
}

/// Executes the body of `defaults.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    globals.set_item(
        "__doc__",
        "Default, keyword, keyword-only, positional-only, *args and **kwargs parameters.",
    )?;
    rt::def!(globals, greet(name, greeting = "Hello", *, punct = "!"))?;
    rt::def!(globals, span(start, stop = py.None(), /, step = 1))?;
    rt::def!(globals, collect(first, *rest, sep = "-", **options))?;
    rt::def!(globals, append_to(item, bucket = PyList::empty(py)))?;
    rt::def!(globals, keyword_only(*, a, b = 2))?;
    rt::def!(globals, scale(x, factor = rt::int(py, 10).mul(2)?))?;
    // Direct calls with omitted defaults, keywords in any order, *args and **kwargs.
    rt::builtins::print(
        py,
        (
            greet(
                py,
                &rt::str(py, "Bob"),
                &rt::default(globals, "greet", "greeting")?,
                &rt::default(globals, "greet", "punct")?,
            )?,
            greet(
                py,
                &rt::str(py, "Al"),
                &rt::str(py, "Hi"),
                &rt::default(globals, "greet", "punct")?,
            )?,
            greet(
                py,
                &rt::str(py, "X"),
                &rt::default(globals, "greet", "greeting")?,
                &rt::str(py, "?"),
            )?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            span(
                py,
                &rt::int(py, 5),
                &rt::default(globals, "span", "stop")?,
                &rt::default(globals, "span", "step")?,
            )?,
            span(
                py,
                &rt::int(py, 2),
                &rt::int(py, 8),
                &rt::default(globals, "span", "step")?,
            )?,
            span(py, &rt::int(py, 2), &rt::int(py, 8), &rt::int(py, 3))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            collect(
                py,
                &rt::int(py, 1),
                &PyTuple::empty(py),
                &rt::default(globals, "collect", "sep")?,
                &PyDict::new(py),
            )?,
            collect(
                py,
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
            append_to(
                py,
                &rt::int(py, 1),
                &rt::default(globals, "append_to", "bucket")?,
            )?,
            append_to(
                py,
                &rt::int(py, 2),
                &rt::default(globals, "append_to", "bucket")?,
            )?,
            append_to(py, &rt::int(py, 3), PyList::empty(py).as_any())?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            keyword_only(
                py,
                &rt::int(py, 1),
                &rt::default(globals, "keyword_only", "b")?,
            )?,
            keyword_only(py, &rt::int(py, 2), &rt::int(py, 5))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (scale(
            py,
            &rt::int(py, 2),
            &rt::default(globals, "scale", "factor")?,
        )?,),
    )?;
    // The same through the function objects.
    globals.set_item("call", rt::global(globals, "greet")?)?;
    rt::builtins::print(
        py,
        (
            rt::global(globals, "call")?.call1(("Bob",))?,
            rt::call!(rt::global(globals, "call")?, name = "Kim", punct = ".")?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            rt::global(globals, "collect")?.getattr("__defaults__")?,
            rt::global(globals, "collect")?.getattr("__kwdefaults__")?,
            rt::global(globals, "greet")?.getattr("__kwdefaults__")?,
        ),
    )?;
    // Defaults are read from the function object at call time.
    let value = PyTuple::new(py, ["Howdy"])?;
    rt::global(globals, "greet")?.setattr("__defaults__", &value)?;
    let value2 = [("punct", "?!")].into_py_dict(py)?;
    rt::global(globals, "greet")?.setattr("__kwdefaults__", &value2)?;
    let value3 = PyTuple::new(py, [3])?;
    rt::global(globals, "scale")?.setattr("__defaults__", &value3)?;
    rt::builtins::print(
        py,
        (
            greet(
                py,
                &rt::str(py, "Sam"),
                &rt::default(globals, "greet", "greeting")?,
                &rt::default(globals, "greet", "punct")?,
            )?,
            rt::global(globals, "call")?.call1(("Sam",))?,
            scale(
                py,
                &rt::int(py, 2),
                &rt::default(globals, "scale", "factor")?,
            )?,
        ),
    )?;
    rt::global(globals, "greet")?.setattr("__kwdefaults__", py.None())?;
    rt::builtins::print(
        py,
        (
            rt::global(globals, "greet")?.getattr("__kwdefaults__")?,
            rt::global(globals, "keyword_only")?.getattr("__defaults__")?,
        ),
    )?;
    rt::builtins::print(
        py,
        (rt::global(globals, "append_to")?.getattr("__defaults__")?,),
    )?;
    Ok(())
}
