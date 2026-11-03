use pyo3::exceptions::PyException;
use pyo3::exceptions::PyKeyError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// except_bad_type.py:6
pub fn report<'py>(m: &rt::Module<'py>, error: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "report", 6, |frame| {
        frame.line(7);
        let mut tb = error.getattr("__traceback__")?;
        frame.line(8);
        while !tb.getattr("tb_next")?.is_none() {
            frame.line(9);
            tb = tb.getattr("tb_next")?;
            py.check_signals()?;
            frame.line(8);
        }
        frame.line(10);
        rt::builtins::print(
            py,
            (
                error.get_type().getattr("__name__")?,
                error.getattr("__context__")?.repr()?,
                tb.getattr("tb_lineno")?,
            ),
        )?;
        Ok(rt::none(py))
    })
}

// except_bad_type.py:13
pub fn bad_class<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "bad_class", 13, |frame| {
        frame.line(15);
        match Err(frame.raise(PyValueError::new_err("x"))) {
            Ok(()) => unreachable!(),
            Err(err) if frame.matches(&err, 16, || Ok(rt::int(py, 42)))? => {}
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

// except_bad_type.py:20
pub fn missing_name<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "missing_name", 20, |frame| {
        frame.line(22);
        let int = rt::builtin(py, "int")?;
        match int.call1(("nope",)) {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyKeyError>(py) => {
                let _handling = rt::Handling::new(&frame.catch(err));
                frame.line(24);
                rt::builtins::print(py, ("not this one",))?;
            }
            Err(err) if frame.matches(&err, 25, || m.global("Missing"))? => {
                // noqa: F821
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

// except_bad_type.py:29
pub fn handled_while_testing<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "handled_while_testing", 29, |frame| {
        // except_bad_type.py:30
        fn pick<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
            let py = m.py();
            rt::frame!(m, "handled_while_testing.<locals>.pick", 30, |frame| {
                frame.line(31);
                rt::builtins::print(
                    py,
                    (
                        "handling",
                        m.global("sys")?.call_method0("exception")?.repr()?,
                    ),
                )?;
                frame.line(32);
                rt::builtin(py, "ValueError")
            })
        }
        let pick = rt::function!(m, pick(), qualname = "handled_while_testing.<locals>.pick")?;
        frame.line(35);
        match Err(frame.raise(PyValueError::new_err("seen"))) {
            Ok(()) => unreachable!(),
            Err(err) if frame.matches(&err, 36, || pick.call0())? => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                frame.line(37);
                rt::builtins::print(py, ("caught", e.repr()?))?;
            }
            Err(err) => return Err(err),
        }
        frame.line(38);
        rt::builtins::print(py, ("after", m.global("sys")?.call_method0("exception")?))?;
        Ok(rt::none(py))
    })
}

// except_bad_type.py:41
pub fn tuple_with_bad_member<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "tuple_with_bad_member", 41, |frame| {
        frame.line(43);
        match rt::int(py, 1).floor_div(n) {
            Ok(value) => Ok(value),
            Err(err)
                if frame.matches(&err, 44, || {
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
    })
}

// except_bad_type.py:48
pub fn class_in_a_local<'py>(
    m: &rt::Module<'py>,
    kinds: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "class_in_a_local", 48, |frame| {
        // The handler expression is evaluated each time, the local stays usable.
        let mut caught = rt::int(py, 0);
        frame.line(51);
        let mut ignored = kinds.get_item(0)?;
        frame.line(52);
        for kind in kinds.try_iter()? {
            let kind = kind?;
            ignored = kind.clone();
            frame.line(55);
            match PyDict::new(py).as_any().get_item("k") {
                Ok(_) => {}
                Err(err) if frame.matches(&err, 56, || Ok(ignored.clone()))? => {
                    let _handling = rt::Handling::new(&frame.catch(err));
                    frame.line(57);
                    caught = rt::iadd(&caught, 1)?;
                }
                Err(err) => return Err(err),
            }
            py.check_signals()?;
            frame.line(52);
        }
        frame.line(58);
        Ok(PyTuple::new(py, [&caught, &ignored.getattr("__name__")?])?.into_any())
    })
}

/// Executes the body of `except_bad_type.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        // Errors raised while matching an `except` clause: a handler that is not an
        // exception class, or a handler expression that raises.
        frame.line(3);
        m.set_global("sys", rt::import(m, "sys")?)?;
        rt::def!(m, report(error))?;
        rt::def!(m, bad_class())?;
        rt::def!(m, missing_name())?;
        rt::def!(m, handled_while_testing())?;
        rt::def!(m, tuple_with_bad_member(n))?;
        rt::def!(m, class_in_a_local(kinds))?;
        frame.line(61);
        // except_bad_type.py:61
        fn lambda<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "<lambda>", 61, |_| tuple_with_bad_member(
                m,
                &rt::int(m.py(), 0)
            ))
        }
        for item in PyTuple::new(
            py,
            [
                &m.global("bad_class")?,
                &m.global("missing_name")?,
                &rt::function!(m, lambda(), qualname = "<lambda>")?,
            ],
        )?
        .try_iter()?
        {
            m.set_global("test", item?)?;
            frame.line(63);
            match (|| m.global("test")?.call0())() {
                Ok(_) => {}
                Err(err) if err.is_instance_of::<PyException>(py) => {
                    let exception = frame.catch(err);
                    m.set_global("error", &exception)?;
                    let _handling = rt::Handling::new(&exception);
                    frame.line(65);
                    report(m, &m.global("error")?)?;
                    m.unbind_global("error")?;
                }
                Err(err) => return Err(err),
            }
            py.check_signals()?;
            frame.line(61);
        }
        frame.line(66);
        handled_while_testing(m)?;
        frame.line(67);
        rt::builtins::print(py, (tuple_with_bad_member(m, &rt::int(py, 1))?,))?;
        frame.line(68);
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
        frame.line(71);
        match PyList::empty(py).as_any().get_item(1) {
            Ok(_) => {}
            Err(err)
                if frame.matches(&err, 72, || {
                    Ok(if m.global("sys")?.getattr("argv")?.len()? > 5 {
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
    })
}
