use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// call_errors.py:6
pub fn report<'py>(py: Python<'py>, args: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::builtins::print(
        py,
        (
            args.getattr("exc_value")?
                .get_type()
                .getattr("__name__")?
                .add(":")?,
            args.getattr("exc_value")?,
        ),
    )?;
    Ok(rt::none(py))
}

// call_errors.py:13
pub fn attempt<'py>(
    py: Python<'py>,
    globals: &Bound<'py, PyDict>,
    f: &Bound<'py, PyAny>,
    args: &Bound<'py, PyTuple>,
    kwargs: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    // A thread reports the exception through `threading.excepthook`.
    let worker = rt::call!(
        rt::global(globals, "threading")?.getattr("Thread")?,
        target = f,
        args = args,
        kwargs = kwargs
    )?;
    worker.call_method0("start")?;
    worker.call_method0("join")?;
    Ok(rt::none(py))
}

// call_errors.py:20
pub fn f<'py>(
    py: Python<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(a.clone())
}

// call_errors.py:24
pub fn g<'py>(
    py: Python<'py>,
    x: &Bound<'py, PyAny>,
    y: &Bound<'py, PyAny>,
    z: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(x.clone())
}

// call_errors.py:28
pub fn h<'py>(
    py: Python<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
    c: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(a.clone())
}

// call_errors.py:32
pub fn k<'py>(
    py: Python<'py>,
    globals_: &Bound<'py, PyAny>,
    r#type: &Bound<'py, PyAny>,
    slf: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(globals_.clone())
}

// call_errors.py:36
pub fn no_args<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::int(py, 0))
}

// call_errors.py:40
pub fn m_<'py>(
    py: Python<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
    c: &Bound<'py, PyAny>,
    d: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(a.clone())
}

/// Executes the body of `call_errors.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    globals.set_item(
        "__doc__",
        "Calls that raise TypeError get CPython's messages, with Python parameter names.",
    )?;
    globals.set_item("threading", rt::import(globals, "threading")?)?;
    rt::def!(globals, report(args))?;
    let report2 = rt::global(globals, "report")?;
    rt::global(globals, "threading")?.setattr("excepthook", &report2)?;
    rt::def!(globals, attempt(globals, f, *args, **kwargs))?;
    rt::def!(globals, f(a, b = 1))?;
    rt::def!(globals, g(x, /, y, *, z))?;
    rt::def!(globals, h(*, a, b, c))?;
    rt::def!(globals, k(globals_ as "globals", r#type, slf as "self"))?;
    rt::def!(globals, no_args())?;
    rt::def!(globals, m_(a, b, c, d), name = "m")?;
    attempt(
        py,
        globals,
        &rt::global(globals, "f")?,
        &PyTuple::empty(py),
        &PyDict::new(py),
    )?;
    attempt(
        py,
        globals,
        &rt::global(globals, "f")?,
        &PyTuple::new(py, [1, 2, 3])?,
        &PyDict::new(py),
    )?;
    attempt(
        py,
        globals,
        &rt::global(globals, "f")?,
        &PyTuple::new(py, [1])?,
        &[("a", 2)].into_py_dict(py)?,
    )?;
    attempt(
        py,
        globals,
        &rt::global(globals, "f")?,
        &PyTuple::new(py, [1])?,
        &[("c", 2)].into_py_dict(py)?,
    )?;
    attempt(
        py,
        globals,
        &rt::global(globals, "f")?,
        &PyTuple::new(py, [1])?,
        &[("bb", 2)].into_py_dict(py)?,
    )?;
    attempt(
        py,
        globals,
        &rt::global(globals, "g")?,
        &PyTuple::empty(py),
        &[("x", 1), ("y", 2), ("z", 3)].into_py_dict(py)?,
    )?;
    attempt(
        py,
        globals,
        &rt::global(globals, "g")?,
        &PyTuple::new(py, [1, 2, 3])?,
        &[("z", 4)].into_py_dict(py)?,
    )?;
    attempt(
        py,
        globals,
        &rt::global(globals, "g")?,
        &PyTuple::new(py, [1, 2])?,
        &PyDict::new(py),
    )?;
    attempt(
        py,
        globals,
        &rt::global(globals, "h")?,
        &PyTuple::empty(py),
        &PyDict::new(py),
    )?;
    attempt(
        py,
        globals,
        &rt::global(globals, "m")?,
        &PyTuple::empty(py),
        &PyDict::new(py),
    )?;
    attempt(
        py,
        globals,
        &rt::global(globals, "m")?,
        &PyTuple::new(py, [1, 2])?,
        &PyDict::new(py),
    )?;
    attempt(
        py,
        globals,
        &rt::global(globals, "k")?,
        &PyTuple::empty(py),
        &PyDict::new(py),
    )?;
    attempt(
        py,
        globals,
        &rt::global(globals, "k")?,
        &PyTuple::new(py, [1, 2, 3])?,
        &[("glob", 4)].into_py_dict(py)?,
    )?;
    attempt(
        py,
        globals,
        &rt::global(globals, "no_args")?,
        &PyTuple::new(py, [1])?,
        &PyDict::new(py),
    )?;
    attempt(
        py,
        globals,
        &rt::global(globals, "no_args")?,
        &PyTuple::new(py, [1, 2])?,
        &[("x", 1)].into_py_dict(py)?,
    )?;
    rt::call!(
        rt::global(globals, "attempt")?,
        rt::builtin(py, "print")?,
        *PyList::new(py, [1])?,
        **[("sep", "-")].into_py_dict(py)?
    )?;
    rt::call!(rt::global(globals, "k")?, 1, 2, "self" = 3, r#type = 4)?;
    Ok(())
}
