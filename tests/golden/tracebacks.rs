//! Parts of `tracebacks.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 31, `def kept_middle`: generator expression with several for clauses

use pyo3::exceptions::PyException;
use pyo3::exceptions::PyIndexError;
use pyo3::exceptions::PyTypeError;
use pyo3::exceptions::PyValueError;
use pyo3::exceptions::PyZeroDivisionError;
use pyo3::prelude::*;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// tracebacks.py:12
pub fn show<'py>(m: &rt::Module<'py>, exc: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "show", 12, |_| {
        for frame_ in m
            .global("traceback")?
            .attr("extract_tb")?
            .call1((exc.attr("__traceback__")?,))?
            .try_iter()?
        {
            let frame_ = frame_?;
            rt::builtins::print(
                py,
                (
                    " ",
                    frame_.attr("name")?,
                    frame_.attr("lineno")?,
                    frame_.attr("line")?,
                ),
            )?;
            py.check_signals()?;
        }
        Ok(rt::none(py))
    })
}

// tracebacks.py:17
pub fn no_carets<'py>(
    m: &rt::Module<'py>,
    text: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "no_carets", 17, |_| {
        // Native frames have no column positions, so no `^^^` markers.
        // tracebacks.py:19
        fn genexpr<'py>(
            m: &rt::Module<'py>,
            iter: &Bound<'py, PyIterator>,
        ) -> PyResult<Option<Bound<'py, PyAny>>> {
            rt::frame!(m, "no_carets.<locals>.<genexpr>", 19, |_| {
                for line in iter {
                    let line = line?;
                    if line.method0("strip")?.method1("strip", ("^~",))?.ne("")? {
                        return Ok(Some(line));
                    }
                    m.py().check_signals()?;
                }
                Ok(None)
            })
        }
        let join = rt::str(m.py(), "\n").attr("join")?;
        let iter = text.method0("splitlines")?.try_iter()?;
        join.call1((rt::genexpr!(
            m,
            genexpr(iter),
            qualname = "no_carets.<locals>.<genexpr>"
        )?,))
    })
}

// tracebacks.py:24
pub fn leaf<'py>(
    m: &rt::Module<'py>,
    values: &Bound<'py, PyAny>,
    key: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "leaf", 24, |_| {
        let mut total = rt::int(py, 0);
        for value in values.try_iter()? {
            let value = value?;
            total = total.add(&value)?;
            py.check_signals()?;
        }
        values.get_item(key)?.add(&total)
    })
}

// tracebacks.py:36
pub fn native_middle<'py>(
    m: &rt::Module<'py>,
    values: &Bound<'py, PyAny>,
    key: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "native_middle", 36, |_| {
        let adjust = rt::int(m.py(), 1);
        let result = m.global("kept_middle")?.call1((values, key))?;
        result.add(&adjust)
    })
}

// tracebacks.py:42
pub fn decorate<'py>(m: &rt::Module<'py>, r#fn: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    Ok(r#fn.clone())
}

// tracebacks.py:47
pub fn decorated<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "decorated", 46, |_| { rt::int(m.py(), 1).div(x) })
}

// tracebacks.py:51
pub fn outer<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "outer", 51, |frame| {
        // tracebacks.py:52
        fn inner<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "outer.<locals>.inner", 52, |_| {
                PyList::new(m.py(), [1])?.as_any().get_item(n)
            })
        }
        let inner = rt::function!(m, inner(n), qualname = "outer.<locals>.inner")?;
        // tracebacks.py:55
        fn lambda<'py>(m: &rt::Module<'py>, v: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "outer.<locals>.<lambda>", 55, |_| { v.mul(v) })
        }
        let square = rt::function!(m, lambda(v), qualname = "outer.<locals>.<lambda>")?;
        match inner.call1((5,)) {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyIndexError>(py) => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                show(m, &e)?;
            }
            Err(err) => return Err(err),
        }
        match square.call1(("text",)) {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyTypeError>(py) => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
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
    rt::frame!(m, "multiline", 67, |_| { a.add(b) })
}

// tracebacks.py:74
pub fn raise_again<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "raise_again", 74, |frame| {
        match (|| leaf(m, PyList::new(py, [1])?.as_any(), &rt::int(py, 3)))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyIndexError>(py) => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
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
        match (|| leaf(m, PyList::new(py, [1])?.as_any(), &rt::int(py, 3)))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyIndexError>(py) => {
                let _handling = rt::Handling::new(&frame.catch(err));
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
    rt::frame!(m, "in_loop", 88, |_| {
        let mut count: Option<Bound<'py, PyAny>> = None;
        for item in items.try_iter()? {
            let item = item?;
            if item.is_truthy()? {
                count = Some(rt::int(py, item.len()?));
            } else {
                count = Some(rt::int(py, 0));
            }
            py.check_signals()?;
        }
        while rt::local(&count, "count")?.lt(3)? {
            count = Some(rt::local(&count, "count")?.add(1)?);
            if rt::local(&count, "count")?.eq(2)? {
                py.check_signals()?;
                continue;
            }
            py.check_signals()?;
        }
        items.get_item(rt::local(&count, "count")?)
    })
}

// tracebacks.py:137
pub fn main<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "main", 137, |_| {
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
        m.set_global("io", rt::import(m, "io")?)?;
        m.set_global("logging", rt::import(m, "logging")?)?;
        m.set_global("sys", rt::import(m, "sys")?)?;
        m.set_global("traceback", rt::import(m, "traceback")?)?;
        m.set_global("log_output", m.global("io")?.method0("StringIO")?)?;
        rt::call!(
            m.global("logging")?.attr("basicConfig")?,
            stream = m.global("log_output")?,
            format = "%(levelname)s %(message)s"
        )?;
        rt::def!(m, show(exc))?;
        rt::def!(m, no_carets(text))?;
        rt::def!(m, leaf(values, key))?;
        // tracebacks.py:31, kept as Python: generator expression with several for clauses at 32:17
        rt::exec_python!(
            m,
            line = 31,
            r#"
def kept_middle(values, key):
    pairs = list(p for p in enumerate(values) for _ in "1")
    return leaf(values, key) + len(pairs)
"#
        )?;
        rt::def!(m, native_middle(values, key))?;
        rt::def!(m, decorate(r#fn))?;
        {
            let decorator = m.global("decorate")?;
            let function = rt::function!(m, decorated(x))?;
            m.set_global("decorated", decorator.call1((function,))?)?;
        }
        rt::def!(m, outer())?;
        rt::def!(m, multiline(a, b))?;
        rt::def!(m, raise_again())?;
        rt::def!(m, raise_from_handler())?;
        rt::def!(m, in_loop(items))?;
        match (|| native_middle(m, PyList::new(py, [1, 2])?.as_any(), &rt::int(py, 7)))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyIndexError>(py) => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                let _handling = rt::Handling::new(&exception);
                show(m, &m.global("e")?)?;
                m.set_global("out", m.global("io")?.method0("StringIO")?)?;
                rt::call!(
                    m.global("traceback")?.attr("print_exc")?,
                    file = m.global("out")?
                )?;
                rt::builtins::print(py, (no_carets(m, &m.global("out")?.method0("getvalue")?)?,))?;
                m.global("logging")?.method1("exception", ("logged",))?;
                rt::builtins::print(
                    py,
                    (no_carets(m, &m.global("log_output")?.method0("getvalue")?)?,),
                )?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        match (|| m.global("decorated")?.call1((0,)))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyZeroDivisionError>(py) => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                let _handling = rt::Handling::new(&exception);
                show(m, &m.global("e")?)?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        outer(m)?;
        match multiline(m, &rt::int(py, 1), &rt::str(py, "x")) {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyTypeError>(py) => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                let _handling = rt::Handling::new(&exception);
                show(m, &m.global("e")?)?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        for item in PyList::new(
            py,
            [&m.global("raise_again")?, &m.global("raise_from_handler")?],
        )?
        .try_iter()?
        {
            m.set_global("fn", item?)?;
            match (|| m.global("fn")?.call0())() {
                Ok(_) => {}
                Err(err) if err.is_instance_of::<PyException>(py) => {
                    let exception = frame.catch(err);
                    m.set_global("e", &exception)?;
                    let _handling = rt::Handling::new(&exception);
                    m.set_global("out", m.global("io")?.method0("StringIO")?)?;
                    rt::call!(
                        m.global("traceback")?.attr("print_exception")?,
                        m.global("e")?,
                        file = m.global("out")?
                    )?;
                    rt::builtins::print(
                        py,
                        (no_carets(m, &m.global("out")?.method0("getvalue")?)?,),
                    )?;
                    m.unbind_global("e")?;
                }
                Err(err) => return Err(err),
            }
            py.check_signals()?;
        }
        match (|| in_loop(m, PyList::new(py, ["ab", ""])?.as_any()))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyIndexError>(py) => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                let _handling = rt::Handling::new(&exception);
                show(m, &m.global("e")?)?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        rt::def!(m, main())?;
        main(m)?;
        Ok(())
    })
}
