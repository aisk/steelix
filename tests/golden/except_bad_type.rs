use pyo3::exceptions::PyException;
use pyo3::exceptions::PyKeyError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// except_bad_type.py:6
#[rt::frame(m, "report", 6)]
pub fn report<'py>(m: &rt::Module<'py>, error: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut tb = error.attr("__traceback__")?;
    while !tb.attr("tb_next")?.is_none() {
        tb = tb.attr("tb_next")?;
        py.check_signals()?;
    }
    rt::builtins::print(
        py,
        (
            error.get_type().attr("__name__")?,
            error.attr("__context__")?.repr()?,
            tb.attr("tb_lineno")?,
        ),
    )?;
    Ok(rt::none(py))
}

// except_bad_type.py:13
#[rt::frame(m, "bad_class", 13, frame)]
pub fn bad_class<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    match Err(frame.raise(PyValueError::new_err("x"))) {
        Ok(()) => unreachable!(),
        Err(err) if frame.matches(&err, || Ok(rt::int(py, 42)))? => {}
        Err(err) => return Err(err),
    }
    Ok(rt::none(py))
}

// except_bad_type.py:20
#[rt::frame(m, "missing_name", 20, frame)]
pub fn missing_name<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let int = rt::builtin(py, "int")?;
    match int.call1(("nope",)) {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyKeyError>(py) => {
            let _caught = frame.catch(err);
            rt::builtins::print(py, ("not this one",))?;
        }
        Err(err) if frame.matches(&err, || m.global("Missing"))? => {
            // noqa: F821
        }
        Err(err) => return Err(err),
    }
    Ok(rt::none(py))
}

// except_bad_type.py:29
#[rt::frame(m, "handled_while_testing", 29, frame)]
pub fn handled_while_testing<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    // except_bad_type.py:30
    #[rt::frame(m, "handled_while_testing.<locals>.pick", 30)]
    fn pick<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::builtins::print(
            py,
            ("handling", m.global("sys")?.method0("exception")?.repr()?),
        )?;
        rt::builtin(py, "ValueError")
    }
    let pick = rt::function!(m, pick(), qualname = "handled_while_testing.<locals>.pick")?;
    match Err(frame.raise(PyValueError::new_err("seen"))) {
        Ok(()) => unreachable!(),
        Err(err) if frame.matches(&err, || pick.call0())? => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("caught", e.repr()?))?;
        }
        Err(err) => return Err(err),
    }
    rt::builtins::print(py, ("after", m.global("sys")?.method0("exception")?))?;
    Ok(rt::none(py))
}

// except_bad_type.py:41
#[rt::frame(m, "tuple_with_bad_member", 41, frame)]
pub fn tuple_with_bad_member<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    match rt::int(py, 1).floor_div(n) {
        Ok(value) => Ok(value),
        Err(err)
            if frame.matches(&err, || {
                Ok(PyTuple::new(
                    py,
                    [&rt::builtin(py, "ArithmeticError")?, &rt::str(py, "text")],
                )?
                .into_any())
            })? =>
        {
            Ok(rt::none(py))
        }
        Err(err) => Err(err),
    }
}

// except_bad_type.py:48
#[rt::frame(m, "class_in_a_local", 48, frame)]
pub fn class_in_a_local<'py>(
    m: &rt::Module<'py>,
    kinds: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    // The handler expression is evaluated each time, the local stays usable.
    let mut caught = rt::int(py, 0);
    let mut ignored = kinds.get_item(0)?;
    for kind in rt::iter(kinds)? {
        let kind = kind?;
        ignored = kind.clone();
        match PyDict::new(py).as_any().get_item("k") {
            Ok(_) => {}
            Err(err) if frame.matches(&err, || Ok(ignored.clone()))? => {
                let _caught = frame.catch(err);
                caught = rt::iadd(&caught, 1)?;
            }
            Err(err) => return Err(err),
        }
    }
    Ok(PyTuple::new(py, [&caught, &ignored.attr("__name__")?])?.into_any())
}

/// Executes the body of `except_bad_type.py` in the module `m`.
#[rt::frame(m, "<module>", 1, frame)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    // Errors raised while matching an `except` clause: a handler that is not an
    // exception class, or a handler expression that raises.
    m.set_global("sys", rt::import(m, "sys")?)?;
    rt::def!(m, report(error))?;
    rt::def!(m, bad_class())?;
    rt::def!(m, missing_name())?;
    rt::def!(m, handled_while_testing())?;
    rt::def!(m, tuple_with_bad_member(n))?;
    rt::def!(m, class_in_a_local(kinds))?;
    // except_bad_type.py:61
    #[rt::frame(m, "<lambda>", 61)]
    fn lambda<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
        tuple_with_bad_member(m, &rt::int(m.py(), 0))
    }
    for item in rt::iter(
        PyTuple::new(
            py,
            [
                &m.global("bad_class")?,
                &m.global("missing_name")?,
                &rt::function!(m, lambda(), qualname = "<lambda>")?,
            ],
        )?
        .as_any(),
    )? {
        m.set_global("test", item?)?;
        match (|| m.global("test")?.call0())() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyException>(py) => {
                let exception = frame.catch(err);
                m.set_global("error", &exception)?;
                report(m, &m.global("error")?)?;
                m.unbind_global("error")?;
            }
            Err(err) => return Err(err),
        }
    }
    handled_while_testing(m)?;
    rt::builtins::print(py, (tuple_with_bad_member(m, &rt::int(py, 1))?,))?;
    rt::builtins::print(
        py,
        (class_in_a_local(
            m,
            PyList::new(
                py,
                [
                    &rt::builtin(py, "KeyError")?,
                    &rt::builtin(py, "LookupError")?,
                ],
            )?
            .as_any(),
        )?,),
    )?;
    match PyList::empty(py).as_any().get_item(1) {
        Ok(_) => {}
        Err(err)
            if frame.matches(&err, || {
                Ok(if m.global("sys")?.attr("argv")?.len()? > 5 {
                    rt::builtin(py, "IndexError")?
                } else {
                    rt::float(py, 3.5)
                })
            })? =>
        {
            // fmt: skip
        }
        Err(err) => return Err(err),
    }
    Ok(())
}
