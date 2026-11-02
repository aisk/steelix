//! Parts of `tracebacks.py` kept as Python source and run by
//! `rt::exec_python!` in `exec_module` (steelix does not lower them yet):
//!
//! - line 17, `def no_carets`: generator expression
//! - line 31, `def kept_middle`: list comprehension

use pyo3::exceptions::PyException;
use pyo3::exceptions::PyIndexError;
use pyo3::exceptions::PyTypeError;
use pyo3::exceptions::PyValueError;
use pyo3::exceptions::PyZeroDivisionError;
use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;

// tracebacks.py:12
pub fn show<'py>(m: &rt::Module<'py>, exc: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "show", 12, |frame| {
        frame.line(13);
        for frame_ in m
            .global("traceback")?
            .getattr("extract_tb")?
            .call1((exc.getattr("__traceback__")?,))?
            .try_iter()?
        {
            let frame_ = frame_?;
            frame.line(14);
            rt::builtins::print(
                py,
                (
                    " ",
                    frame_.getattr("name")?,
                    frame_.getattr("lineno")?,
                    frame_.getattr("line")?,
                ),
            )?;
            py.check_signals()?;
            frame.line(13);
        }
        Ok(rt::none(py))
    })
}

// tracebacks.py:24
pub fn leaf<'py>(
    m: &rt::Module<'py>,
    values: &Bound<'py, PyAny>,
    key: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "leaf", 24, |frame| {
        let mut total = rt::int(py, 0);
        frame.line(26);
        for value in values.try_iter()? {
            let value = value?;
            frame.line(27);
            total = total.add(&value)?;
            py.check_signals()?;
            frame.line(26);
        }
        frame.line(28);
        values.get_item(key)?.add(&total)
    })
}

// tracebacks.py:36
pub fn native_middle<'py>(
    m: &rt::Module<'py>,
    values: &Bound<'py, PyAny>,
    key: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "native_middle", 36, |frame| {
        let adjust = rt::int(m.py(), 1);
        frame.line(38);
        let result = m.global("kept_middle")?.call1((values, key))?;
        frame.line(39);
        result.add(&adjust)
    })
}

// tracebacks.py:42
pub fn decorate<'py>(m: &rt::Module<'py>, r#fn: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    Ok(r#fn.clone())
}

// tracebacks.py:47
pub fn decorated<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "decorated", 46, |frame| {
        frame.line(48);
        rt::int(m.py(), 1).div(x)
    })
}

// tracebacks.py:51
pub fn outer<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "outer", 51, |frame| {
        // tracebacks.py:52
        fn inner<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "outer.<locals>.inner", 52, |frame| {
                frame.line(53);
                PyList::new(m.py(), [1])?.as_any().get_item(n)
            })
        }
        let inner = rt::function!(m, inner(n), qualname = "outer.<locals>.inner")?;
        // tracebacks.py:55
        fn lambda<'py>(m: &rt::Module<'py>, v: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "outer.<locals>.<lambda>", 55, |_| v.mul(v))
        }
        frame.line(55);
        let square = rt::function!(m, lambda(v), qualname = "outer.<locals>.<lambda>")?;
        frame.line(57);
        match inner.call1((5,)) {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyIndexError>(py) => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                frame.line(59);
                show(m, &e)?;
            }
            Err(err) => return Err(err),
        }
        frame.line(61);
        match square.call1(("text",)) {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyTypeError>(py) => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                frame.line(63);
                show(m, &e)?;
            }
            Err(err) => return Err(err),
        }
        Ok(inner)
    })
}

// tracebacks.py:67
pub fn multiline<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "multiline", 67, |frame| {
        frame.line(69);
        a.add(b)
    })
}

// tracebacks.py:74
pub fn raise_again<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "raise_again", 74, |frame| {
        frame.line(76);
        match (|| leaf(m, PyList::new(py, [1])?.as_any(), &rt::int(py, 3)))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyIndexError>(py) => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                frame.line(78);
                return Err(frame.raise(&e));
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

// tracebacks.py:81
pub fn raise_from_handler<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "raise_from_handler", 81, |frame| {
        frame.line(83);
        match (|| leaf(m, PyList::new(py, [1])?.as_any(), &rt::int(py, 3)))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyIndexError>(py) => {
                let _handling = rt::Handling::new(&frame.catch(err));
                frame.line(85);
                return Err(frame.raise(PyValueError::new_err("converted")));
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

// tracebacks.py:88
pub fn in_loop<'py>(m: &rt::Module<'py>, items: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "in_loop", 88, |frame| {
        let mut count: Option<Bound<'py, PyAny>> = None;
        frame.line(89);
        for item in items.try_iter()? {
            let item = item?;
            frame.line(90);
            if item.is_truthy()? {
                frame.line(91);
                count = Some(rt::int(py, item.len()?));
            } else {
                count = Some(rt::int(py, 0));
            }
            py.check_signals()?;
            frame.line(89);
        }
        frame.line(94);
        while rt::local(&count, "count")?.lt(3)? {
            frame.line(95);
            count = Some(rt::local(&count, "count")?.add(1)?);
            frame.line(96);
            if rt::local(&count, "count")?.eq(2)? {
                py.check_signals()?;
                frame.line(94);
                continue;
            }
            py.check_signals()?;
            frame.line(94);
        }
        frame.line(98);
        items.get_item(rt::local(&count, "count")?)
    })
}

// tracebacks.py:137
pub fn main<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "main", 137, |frame| {
        frame.line(138);
        native_middle(m, PyList::new(py, [3])?.as_any(), &rt::int(py, 9))?;
        Ok(rt::none(py))
    })
}

/// Executes the body of `tracebacks.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        // Traceback entries of native functions: lines, names, and the order of
        // native and kept (Python) frames, printed by the traceback module.
        frame.line(3);
        m.set_global("io", rt::import(m, "io")?)?;
        frame.line(4);
        m.set_global("logging", rt::import(m, "logging")?)?;
        frame.line(5);
        m.set_global("sys", rt::import(m, "sys")?)?;
        frame.line(6);
        m.set_global("traceback", rt::import(m, "traceback")?)?;
        frame.line(8);
        m.set_global("log_output", m.global("io")?.call_method0("StringIO")?)?;
        frame.line(9);
        rt::call!(
            m.global("logging")?.getattr("basicConfig")?,
            stream = m.global("log_output")?,
            format = "%(levelname)s %(message)s"
        )?;
        rt::def!(m, show(exc))?;
        // tracebacks.py:17, kept as Python: generator expression at 19:21
        rt::exec_python!(
            m,
            line = 17,
            r#"
def no_carets(text):
    # Native frames have no column positions, so no `^^^` markers.
    return "\n".join(
        line for line in text.splitlines() if line.strip().strip("^~") != ""
    )
"#
        )?;
        rt::def!(m, leaf(values, key))?;
        // tracebacks.py:31, kept as Python: list comprehension at 32:13
        rt::exec_python!(
            m,
            line = 31,
            r#"
def kept_middle(values, key):
    pairs = [(k, v) for k, v in enumerate(values)]
    return leaf(values, key) + len(pairs)
"#
        )?;
        rt::def!(m, native_middle(values, key))?;
        rt::def!(m, decorate(r#fn))?;
        frame.line(47);
        let decorator = m.global("decorate")?;
        let function = rt::function!(m, decorated(x))?;
        m.set_global("decorated", decorator.call1((&function,))?)?;
        rt::def!(m, outer())?;
        rt::def!(m, multiline(a, b))?;
        rt::def!(m, raise_again())?;
        rt::def!(m, raise_from_handler())?;
        rt::def!(m, in_loop(items))?;
        frame.line(102);
        match (|| native_middle(m, PyList::new(py, [1, 2])?.as_any(), &rt::int(py, 7)))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyIndexError>(py) => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(104);
                show(m, &m.global("e")?)?;
                frame.line(105);
                m.set_global("out", m.global("io")?.call_method0("StringIO")?)?;
                frame.line(106);
                rt::call!(
                    m.global("traceback")?.getattr("print_exc")?,
                    file = m.global("out")?
                )?;
                frame.line(107);
                rt::builtins::print(
                    py,
                    (m.global("no_carets")?
                        .call1((m.global("out")?.call_method0("getvalue")?,))?,),
                )?;
                frame.line(108);
                m.global("logging")?
                    .call_method1("exception", ("logged",))?;
                frame.line(109);
                rt::builtins::print(
                    py,
                    (m.global("no_carets")?
                        .call1((m.global("log_output")?.call_method0("getvalue")?,))?,),
                )?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        frame.line(112);
        match (|| m.global("decorated")?.call1((0,)))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyZeroDivisionError>(py) => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(114);
                show(m, &m.global("e")?)?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        frame.line(116);
        outer(m)?;
        frame.line(119);
        match multiline(m, &rt::int(py, 1), &rt::str(py, "x")) {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyTypeError>(py) => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(121);
                show(m, &m.global("e")?)?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        frame.line(123);
        for item in PyList::new(
            py,
            [&m.global("raise_again")?, &m.global("raise_from_handler")?],
        )?
        .try_iter()?
        {
            m.set_global("fn", item?)?;
            frame.line(125);
            match (|| m.global("fn")?.call0())() {
                Ok(_) => {}
                Err(err) if err.is_instance_of::<PyException>(py) => {
                    let exception = frame.catch(err);
                    m.set_global("e", &exception)?;
                    let _handling = rt::Handling::new(&exception);
                    frame.line(127);
                    m.set_global("out", m.global("io")?.call_method0("StringIO")?)?;
                    frame.line(128);
                    rt::call!(
                        m.global("traceback")?.getattr("print_exception")?,
                        m.global("e")?,
                        file = m.global("out")?
                    )?;
                    frame.line(129);
                    rt::builtins::print(
                        py,
                        (m.global("no_carets")?
                            .call1((m.global("out")?.call_method0("getvalue")?,))?,),
                    )?;
                    m.unbind_global("e")?;
                }
                Err(err) => return Err(err),
            }
            py.check_signals()?;
            frame.line(123);
        }
        frame.line(132);
        match (|| in_loop(m, PyList::new(py, ["ab", ""])?.as_any()))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyIndexError>(py) => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(134);
                show(m, &m.global("e")?)?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        rt::def!(m, main())?;
        frame.line(141);
        main(m)?;
        Ok(())
    })
}
