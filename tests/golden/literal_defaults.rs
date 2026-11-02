use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;

// literal_defaults.py:4
pub fn greet<'py>(
    m: &rt::Module<'py>,
    name: &Bound<'py, PyAny>,
    greeting: &Bound<'py, PyAny>,
    punct: &Bound<'py, PyAny>,
    times: &Bound<'py, PyAny>,
    sep: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut sep = sep.clone();
    let text = greeting.add(", ")?.add(name)?.add(punct)?;
    if sep.is_none() {
        sep = rt::str(py, " ");
    }
    sep.getattr("join")?
        .call1((PyList::new(py, [&text])?.mul(times)?,))
}

// literal_defaults.py:11
pub fn scaled<'py>(
    m: &rt::Module<'py>,
    x: &Bound<'py, PyAny>,
    factor: &Bound<'py, PyAny>,
    flag: &Bound<'py, PyAny>,
    raw: &Bound<'py, PyAny>,
    big: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(PyList::new(m.py(), [&x.mul(factor)?, flag, raw, big])?.into_any())
}

// literal_defaults.py:15
pub fn remember<'py>(
    m: &rt::Module<'py>,
    item: &Bound<'py, PyAny>,
    seen: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    // A mutable default is still read from the function object.
    seen.call_method1("append", (item,))?;
    Ok(seen.clone())
}

/// Executes the body of `literal_defaults.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "Direct calls pass immutable literal defaults in place.",
    )?;
    rt::def!(
        m,
        greet(name, greeting = "Hello", *, punct = "!", times = 1, sep = py.None())
    )?;
    rt::def!(
        m,
        scaled(
            x,
            factor = 2.5,
            flag = true,
            raw = rt::bytes(py, b"\0"),
            big = rt::int(py, 10).pow(20, py.None())?
        )
    )?;
    rt::def!(m, remember(item, seen = PyList::empty(py)))?;
    rt::builtins::print(
        py,
        (
            greet(
                m,
                &rt::str(py, "Bob"),
                &rt::str(py, "Hello"),
                &rt::str(py, "!"),
                &rt::int(py, 1),
                &rt::none(py),
            )?,
            greet(
                m,
                &rt::str(py, "Al"),
                &rt::str(py, "Hello"),
                &rt::str(py, "!"),
                &rt::int(py, 2),
                &rt::none(py),
            )?,
            greet(
                m,
                &rt::str(py, "X"),
                &rt::str(py, "Hi"),
                &rt::str(py, "!"),
                &rt::int(py, 2),
                &rt::str(py, "-"),
            )?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            scaled(
                m,
                &rt::int(py, 2),
                &rt::float(py, 2.5),
                &rt::bool(py, true),
                &rt::bytes(py, b"\0"),
                &m.default("scaled", "big")?,
            )?,
            scaled(
                m,
                &rt::int(py, 1),
                &rt::int(py, 4),
                &rt::bool(py, true),
                &rt::bytes(py, b"\0"),
                &m.default("scaled", "big")?,
            )?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            remember(m, &rt::int(py, 1), &m.default("remember", "seen")?)?,
            remember(m, &rt::int(py, 2), &m.default("remember", "seen")?)?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("greet")?.getattr("__defaults__")?,
            m.global("greet")?.getattr("__kwdefaults__")?,
            m.global("scaled")?.getattr("__defaults__")?,
        ),
    )?;
    Ok(())
}
