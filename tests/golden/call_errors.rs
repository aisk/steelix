use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// call_errors.py:6
#[rt::frame(m, "report", 6)]
pub fn report<'py>(m: &rt::Module<'py>, args: &Bound<'py, PyAny>) -> PyResult<()> {
    rt::builtins::print(
        m.py(),
        (
            args.attr("exc_value")?
                .get_type()
                .attr("__name__")?
                .add(":")?,
            args.attr("exc_value")?,
        ),
    )?;
    Ok(())
}

// call_errors.py:13
#[rt::frame(m, "attempt", 13)]
pub fn attempt<'py>(
    m: &rt::Module<'py>,
    f: &Bound<'py, PyAny>,
    args: &Bound<'py, PyTuple>,
    kwargs: &Bound<'py, PyDict>,
) -> PyResult<()> {
    // A thread reports the exception through `threading.excepthook`.
    let worker = rt::call!(
        m.global("threading")?.attr("Thread")?,
        target = f,
        args = args,
        kwargs = kwargs
    )?;
    worker.method0("start")?;
    worker.method0("join")?;
    Ok(())
}

// call_errors.py:20
pub fn f<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(a.clone())
}

// call_errors.py:24
pub fn g<'py>(
    m: &rt::Module<'py>,
    x: &Bound<'py, PyAny>,
    y: &Bound<'py, PyAny>,
    z: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(x.clone())
}

// call_errors.py:28
pub fn h<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
    c: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(a.clone())
}

// call_errors.py:32
pub fn k<'py>(
    m: &rt::Module<'py>,
    globals: &Bound<'py, PyAny>,
    r#type: &Bound<'py, PyAny>,
    slf: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(globals.clone())
}

// call_errors.py:36
pub fn no_args<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::int(m.py(), 0))
}

// call_errors.py:40
pub fn m_<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
    c: &Bound<'py, PyAny>,
    d: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(a.clone())
}

/// Executes the body of `call_errors.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "Calls that raise TypeError get CPython's messages, with Python parameter names.",
    )?;
    m.set_global("threading", rt::import(m, "threading")?)?;
    rt::def!(m, report(args))?;
    let report2 = m.global("report")?;
    m.global("threading")?.set_attr("excepthook", report2)?;
    rt::def!(m, attempt(f, *args, **kwargs))?;
    rt::def!(m, f(a, b = 1))?;
    rt::def!(m, g(x, /, y, *, z))?;
    rt::def!(m, h(*, a, b, c))?;
    rt::def!(m, k(globals, r#type, slf as "self"))?;
    rt::def!(m, no_args())?;
    rt::def!(m, m_ as "m"(a, b, c, d))?;
    attempt(m, &m.global("f")?, &PyTuple::empty(py), &PyDict::new(py))?;
    attempt(
        m,
        &m.global("f")?,
        &PyTuple::new(py, [1, 2, 3])?,
        &PyDict::new(py),
    )?;
    attempt(
        m,
        &m.global("f")?,
        &PyTuple::new(py, [1])?,
        &[("a", 2)].into_py_dict(py)?,
    )?;
    attempt(
        m,
        &m.global("f")?,
        &PyTuple::new(py, [1])?,
        &[("c", 2)].into_py_dict(py)?,
    )?;
    attempt(
        m,
        &m.global("f")?,
        &PyTuple::new(py, [1])?,
        &[("bb", 2)].into_py_dict(py)?,
    )?;
    attempt(
        m,
        &m.global("g")?,
        &PyTuple::empty(py),
        &[("x", 1), ("y", 2), ("z", 3)].into_py_dict(py)?,
    )?;
    attempt(
        m,
        &m.global("g")?,
        &PyTuple::new(py, [1, 2, 3])?,
        &[("z", 4)].into_py_dict(py)?,
    )?;
    attempt(
        m,
        &m.global("g")?,
        &PyTuple::new(py, [1, 2])?,
        &PyDict::new(py),
    )?;
    attempt(m, &m.global("h")?, &PyTuple::empty(py), &PyDict::new(py))?;
    attempt(m, &m.global("m")?, &PyTuple::empty(py), &PyDict::new(py))?;
    attempt(
        m,
        &m.global("m")?,
        &PyTuple::new(py, [1, 2])?,
        &PyDict::new(py),
    )?;
    attempt(m, &m.global("k")?, &PyTuple::empty(py), &PyDict::new(py))?;
    attempt(
        m,
        &m.global("k")?,
        &PyTuple::new(py, [1, 2, 3])?,
        &[("glob", 4)].into_py_dict(py)?,
    )?;
    attempt(
        m,
        &m.global("no_args")?,
        &PyTuple::new(py, [1])?,
        &PyDict::new(py),
    )?;
    attempt(
        m,
        &m.global("no_args")?,
        &PyTuple::new(py, [1, 2])?,
        &[("x", 1)].into_py_dict(py)?,
    )?;
    rt::call!(
        m.global("attempt")?,
        rt::builtin(py, "print")?,
        *PyList::new(py, [1])?,
        **[("sep", "-")].into_py_dict(py)?
    )?;
    rt::call!(m.global("k")?, 1, 2, "self" = 3, r#type = 4)?;
    Ok(())
}
