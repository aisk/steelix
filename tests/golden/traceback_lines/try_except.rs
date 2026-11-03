//! Parts of `try_except.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 196, `def gen`: yield
//! - line 222, `def groups`: try/except*
//! - line 232, `def name_after_handler`: except-as name used outside its handler

use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// try_except.py:13
pub fn parse<'py>(m: &rt::Module<'py>, s: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "parse", 13, |frame| {
        frame.line(15);
        let int = m.global("int")?;
        let x = match int.call1((s,)) {
            Ok(value) => value,
            Err(err) if frame.matches(&err, 16, || m.global("ValueError"))? => rt::int(m.py(), 0),
            Err(err) => return Err(err),
        };
        Ok(x)
    })
}

// try_except.py:21
pub fn parse_or_none<'py>(
    m: &rt::Module<'py>,
    s: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "parse_or_none", 21, |frame| {
        frame.line(23);
        let int = m.global("int")?;
        match int.call1((s,)) {
            Ok(value) => Ok(value),
            Err(err)
                if frame.matches(&err, 24, || {
                    Ok(
                        PyTuple::new(py, [&m.global("ValueError")?, &m.global("TypeError")?])?
                            .into_any(),
                    )
                })? =>
            {
                Ok(rt::none(py))
            }
            Err(err) => Err(err),
        }
    })
}

// try_except.py:28
pub fn describe<'py>(m: &rt::Module<'py>, exc: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "describe", 28, |frame| {
        frame.line(29);
        m.global("print")?.call1((
            m.global("type")?.call1((exc,))?.attr("__name__")?,
            exc.attr("args")?,
            "cause:",
            m.global("repr")?.call1((exc.attr("__cause__")?,))?,
            "context:",
            m.global("repr")?.call1((exc.attr("__context__")?,))?,
            "suppress:",
            exc.attr("__suppress_context__")?,
        ))?;
        Ok(rt::none(m.py()))
    })
}

// try_except.py:33
pub fn risky<'py>(m: &rt::Module<'py>, kind: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "risky", 33, |frame| {
        frame.line(34);
        if kind.eq("value")? {
            frame.line(35);
            return Err(frame.raise(m.global("ValueError")?.call1(("bad value",))?));
        }
        frame.line(36);
        if kind.eq("key")? {
            frame.line(37);
            return Err(frame.raise(m.global("KeyError")?.call1((kind,))?));
        }
        frame.line(38);
        if kind.eq("app")? {
            frame.line(39);
            return Err(frame.raise(m.global("SubError")?.call1(("sub", 2))?));
        }
        frame.line(40);
        if kind.eq("class")? {
            frame.line(41);
            return Err(frame.raise(m.global("AppError")?));
        }
        Ok(kind.clone())
    })
}

// try_except.py:45
pub fn classify<'py>(m: &rt::Module<'py>, kind: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "classify", 45, |frame| {
        let result2 = (|| -> PyResult<Bound<'py, PyAny>> {
            frame.line(47);
            let result: Bound<'py, PyAny>;
            match (|| m.global("risky")?.call1((kind,)))() {
                Ok(value) => {
                    result = value;
                    frame.line(56);
                    m.global("print")?.call1(("no error, result", &result))?;
                    Ok(rt::str(py, "ok"))
                }
                Err(err) if frame.matches(&err, 48, || m.global("KeyError"))? => {
                    let e = frame.catch(err);
                    let _handling = rt::Handling::new(&e);
                    frame.line(49);
                    m.global("print")?.call1(("key error", &e))?;
                    Ok(rt::str(py, "key"))
                }
                Err(err)
                    if frame.matches(&err, 51, || {
                        Ok(
                            PyTuple::new(py, [&m.global("ValueError")?, &m.global("AppError")?])?
                                .into_any(),
                        )
                    })? =>
                {
                    let e = frame.catch(err);
                    let _handling = rt::Handling::new(&e);
                    frame.line(52);
                    m.global("print")?.call1((
                        "caught",
                        m.global("type")?.call1((&e,))?.attr("__name__")?,
                        e.attr("args")?,
                    ))?;
                    frame.line(53);
                    m.global("print")?.call1((
                        "sys.exception() is e:",
                        m.global("sys")?.method0("exception")?.is(&e),
                    ))?;
                    Ok(rt::str(py, "other"))
                }
                Err(err) => Err(err),
            }
        })();
        let finally = frame.finally(result2);
        frame.line(59);
        m.global("print")?.call1(("finally for", kind))?;
        finally.resume()
    })
}

// try_except.py:62
pub fn chained<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "chained", 62, |frame| {
        match (|| -> PyResult<()> {
            frame.line(65);
            match (|| m.global("risky")?.call1(("value",)))() {
                Ok(_) => {}
                Err(err) if frame.matches(&err, 66, || m.global("ValueError"))? => {
                    let e = frame.catch(err);
                    let _handling = rt::Handling::new(&e);
                    frame.line(67);
                    return Err(
                        frame.raise_from(m.global("RuntimeError")?.call1(("wrapped",))?, &e)
                    );
                }
                Err(err) => return Err(err),
            }
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if frame.matches(&err, 68, || m.global("RuntimeError"))? => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                frame.line(69);
                m.global("describe")?.call1((&e,))?;
            }
            Err(err) => return Err(err),
        }
        match (|| -> PyResult<()> {
            frame.line(72);
            match (|| m.global("risky")?.call1(("key",)))() {
                Ok(_) => {}
                Err(err) if frame.matches(&err, 73, || m.global("KeyError"))? => {
                    let _handling = rt::Handling::new(&frame.catch(err));
                    frame.line(74);
                    return Err(frame.raise(m.global("RuntimeError")?.call1(("implicit",))?));
                }
                Err(err) => return Err(err),
            }
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if frame.matches(&err, 75, || m.global("RuntimeError"))? => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                frame.line(76);
                m.global("describe")?.call1((&e,))?;
            }
            Err(err) => return Err(err),
        }
        match (|| -> PyResult<()> {
            frame.line(79);
            match (|| m.global("risky")?.call1(("app",)))() {
                Ok(_) => {}
                Err(err) if frame.matches(&err, 80, || m.global("AppError"))? => {
                    let _handling = rt::Handling::new(&frame.catch(err));
                    frame.line(81);
                    return Err(
                        frame.raise_from(m.global("TypeError")?.call1(("hidden",))?, py.None())
                    );
                }
                Err(err) => return Err(err),
            }
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if frame.matches(&err, 82, || m.global("TypeError"))? => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                frame.line(83);
                m.global("describe")?.call1((&e,))?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

// try_except.py:86
pub fn reraise<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "reraise", 86, |frame| {
        frame.line(88);
        match (|| m.global("risky")?.call1(("value",)))() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 89, || m.global("ValueError"))? => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                frame.line(90);
                e.set_attr("args", PyTuple::new(py, ["changed"])?)?;
                frame.line(91);
                return Err(frame.reraise());
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

// try_except.py:94
pub fn bare_reraise_helper<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "bare_reraise_helper", 94, |frame| {
        frame.line(95);
        Err(frame.reraise())
    })
}

// try_except.py:98
pub fn via_helper<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "via_helper", 98, |frame| {
        frame.line(100);
        match (|| m.global("risky")?.call1(("key",)))() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 101, || m.global("KeyError"))? => {
                let _handling = rt::Handling::new(&frame.catch(err));
                frame.line(102);
                m.global("bare_reraise_helper")?.call0()?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(m.py()))
    })
}

// try_except.py:105
pub fn no_active<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "no_active", 105, |frame| {
        frame.line(107);
        match Err(frame.reraise()) {
            Ok(()) => unreachable!(),
            Err(err) if frame.matches(&err, 108, || m.global("RuntimeError"))? => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                frame.line(109);
                m.global("print")?.call1(("RuntimeError:", &e))?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(m.py()))
    })
}

// try_except.py:112
pub fn not_an_exception<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "not_an_exception", 112, |frame| {
        frame.line(114);
        match Err(frame.raise(rt::int(py, 42))) {
            Ok(()) => unreachable!(),
            Err(err) if frame.matches(&err, 115, || m.global("TypeError"))? => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                frame.line(116);
                m.global("print")?.call1(("TypeError:", &e))?;
            }
            Err(err) => return Err(err),
        }
        match (|| -> PyResult<()> {
            frame.line(118);
            Err(frame.raise_from(m.global("ValueError")?, 3))
        })() {
            Ok(()) => unreachable!(),
            Err(err) if frame.matches(&err, 119, || m.global("TypeError"))? => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                frame.line(120);
                m.global("print")?.call1(("TypeError:", &e))?;
            }
            Err(err) => return Err(err),
        }
        match (|| -> PyResult<()> {
            let result: PyResult<()> = Ok(());
            let finally = frame.finally(result);
            frame.line(125);
            m.global("risky")?.call1(("value",))?;
            finally.resume()?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if frame.matches(&err, 126, || m.global("int"))? => {
                let _handling = rt::Handling::new(&frame.catch(err));
                frame.line(127);
                m.global("print")?.call1(("unreachable",))?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

// try_except.py:130
pub fn bad_handler<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "bad_handler", 130, |frame| {
        frame.line(132);
        match (|| m.global("not_an_exception")?.call0())() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 133, || m.global("TypeError"))? => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                frame.line(134);
                m.global("print")?.call1(("bad handler:", &e))?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(m.py()))
    })
}

// try_except.py:137
pub fn handled_state<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "handled_state", 137, |frame| {
        frame.line(138);
        m.global("print")?
            .call1(("outside:", m.global("sys")?.method0("exception")?))?;
        frame.line(140);
        match (|| m.global("risky")?.call1(("value",)))() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 141, || m.global("ValueError"))? => {
                let _handling = rt::Handling::new(&frame.catch(err));
                frame.line(142);
                m.global("print")?.call1((
                    "inside:",
                    m.global("repr")?
                        .call1((m.global("sys")?.method0("exception")?,))?,
                ))?;
                frame.line(144);
                match (|| m.global("risky")?.call1(("key",)))() {
                    Ok(_) => {}
                    Err(err) if frame.matches(&err, 145, || m.global("KeyError"))? => {
                        let _handling = rt::Handling::new(&frame.catch(err));
                        frame.line(146);
                        m.global("print")?.call1((
                            "nested:",
                            m.global("repr")?
                                .call1((m.global("sys")?.method0("exception")?,))?,
                        ))?;
                    }
                    Err(err) => return Err(err),
                }
                frame.line(147);
                m.global("print")?.call1((
                    "back:",
                    m.global("repr")?
                        .call1((m.global("sys")?.method0("exception")?,))?,
                ))?;
            }
            Err(err) => return Err(err),
        }
        frame.line(148);
        m.global("print")?
            .call1(("after:", m.global("sys")?.method0("exception")?))?;
        Ok(rt::none(m.py()))
    })
}

// try_except.py:151
pub fn handler_name_is_deleted<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "handler_name_is_deleted", 151, |frame| {
        frame.line(153);
        match (|| m.global("risky")?.call1(("value",)))() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 154, || m.global("ValueError"))? => {
                let err = frame.catch(err);
                let _handling = rt::Handling::new(&err);
                frame.line(155);
                m.global("print")?.call1(("bound:", &err))?;
            }
            Err(err2) => return Err(err2),
        }
        Ok(rt::str(m.py(), "done"))
    })
}

// try_except.py:159
pub fn bare_except_catches_everything<'py>(
    m: &rt::Module<'py>,
    exc: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "bare_except_catches_everything", 159, |frame| {
        frame.line(161);
        match Err(frame.raise(exc)) {
            Ok(()) => unreachable!(),
            Err(err) if frame.matches(&err, 162, || m.global("Exception"))? => {
                Ok(rt::str(py, "Exception"))
            }
            Err(_) => Ok(rt::str(py, "bare")),
        }
    })
}

// try_except.py:168
pub fn exceptions_in_else_and_handlers<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "exceptions_in_else_and_handlers", 168, |frame| {
        match (|| -> PyResult<()> {
            frame.line(171);
            let print = m.global("print")?;
            match print.call1(("body",)) {
                Ok(_) => {
                    frame.line(175);
                    m.global("risky")?.call1(("key",))?;
                }
                Err(err) if frame.matches(&err, 172, || m.global("ValueError"))? => {
                    let _handling = rt::Handling::new(&frame.catch(err));
                    frame.line(173);
                    m.global("print")?.call1(("not reached",))?;
                }
                Err(err) => return Err(err),
            }
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if frame.matches(&err, 176, || m.global("KeyError"))? => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                frame.line(177);
                m.global("print")?
                    .call1(("from else:", m.global("repr")?.call1((&e,))?))?;
            }
            Err(err) => return Err(err),
        }
        match (|| -> PyResult<()> {
            frame.line(180);
            match (|| m.global("risky")?.call1(("value",)))() {
                Ok(_) => {}
                Err(err) if frame.matches(&err, 181, || m.global("ValueError"))? => {
                    let _handling = rt::Handling::new(&frame.catch(err));
                    frame.line(182);
                    m.global("risky")?.call1(("key",))?;
                }
                Err(err) => return Err(err),
            }
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if frame.matches(&err, 183, || m.global("KeyError"))? => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                frame.line(184);
                m.global("print")?.call1((
                    "from handler:",
                    m.global("repr")?.call1((&e,))?,
                    "context:",
                    m.global("repr")?.call1((e.attr("__context__")?,))?,
                ))?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(m.py()))
    })
}

// try_except.py:187
pub fn stop_iteration<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "stop_iteration", 187, |frame| {
        frame.line(188);
        let it = m.global("iter")?.call1((PyList::new(py, [1])?,))?;
        match (|| -> PyResult<()> {
            frame.line(190);
            m.global("next")?.call1((&it,))?;
            frame.line(191);
            m.global("next")?.call1((&it,))?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if frame.matches(&err, 192, || m.global("StopIteration"))? => {
                let _handling = rt::Handling::new(&frame.catch(err));
                frame.line(193);
                m.global("print")?.call1(("exhausted",))?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

// try_except.py:201
pub fn generator_into_native<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "generator_into_native", 201, |frame| {
        let mut total = rt::int(py, 0);
        match (|| -> PyResult<()> {
            frame.line(204);
            for value in m.global("gen")?.call0()?.try_iter()? {
                let value = value?;
                frame.line(205);
                total = total.add(&value)?;
                py.check_signals()?;
                frame.line(204);
            }
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if frame.matches(&err, 206, || m.global("ValueError"))? => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                frame.line(207);
                m.global("print")?
                    .call1(("generator raised", &e, "after", &total))?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

// try_except.py:210
pub fn loop_with_try<'py>(
    m: &rt::Module<'py>,
    items: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "loop_with_try", 210, |frame| {
        let mut total = rt::int(py, 0);
        frame.line(212);
        for item in items.try_iter()? {
            let item = item?;
            frame.line(214);
            match (|| total.add(m.global("int")?.call1((&item,))?))() {
                Ok(value) => total = value,
                Err(err) if frame.matches(&err, 215, || m.global("ValueError"))? => {
                    py.check_signals()?;
                    frame.line(212);
                    continue;
                }
                Err(err) => return Err(err),
            }
            frame.line(217);
            if total.gt(10)? {
                break;
            }
            py.check_signals()?;
            frame.line(212);
        }
        Ok(total)
    })
}

pub mod app_error {
    use super::*;

    // try_except.py:5
    /// The body of `class AppError`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

pub mod sub_error {
    use super::*;

    // try_except.py:9
    /// The body of `class SubError`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

/// Executes the body of `try_except.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        // try/except in native functions: matching, `as` names, `else`, chaining.
        frame.line(2);
        m.set_global("sys", rt::import(m, "sys")?)?;
        frame.line(5);
        m.set_global(
            "AppError",
            rt::class!(
                m,
                "AppError"(m.global("Exception")?),
                app_error::body,
                line = 5
            )?,
        )?;
        frame.line(9);
        m.set_global(
            "SubError",
            rt::class!(
                m,
                "SubError"(m.global("AppError")?),
                sub_error::body,
                line = 9
            )?,
        )?;
        rt::def!(m, parse(s))?;
        rt::def!(m, parse_or_none(s))?;
        rt::def!(m, describe(exc))?;
        rt::def!(m, risky(kind))?;
        rt::def!(m, classify(kind))?;
        rt::def!(m, chained())?;
        rt::def!(m, reraise())?;
        rt::def!(m, bare_reraise_helper())?;
        rt::def!(m, via_helper())?;
        rt::def!(m, no_active())?;
        rt::def!(m, not_an_exception())?;
        rt::def!(m, bad_handler())?;
        rt::def!(m, handled_state())?;
        rt::def!(m, handler_name_is_deleted())?;
        rt::def!(m, bare_except_catches_everything(exc))?;
        rt::def!(m, exceptions_in_else_and_handlers())?;
        rt::def!(m, stop_iteration())?;
        // try_except.py:196, kept as Python: yield at 197:5
        rt::exec_python!(
            m,
            line = 196,
            r#"
def gen():
    yield 1
    raise ValueError("from generator")
"#
        )?;
        rt::def!(m, generator_into_native())?;
        rt::def!(m, loop_with_try(items))?;
        // try_except.py:222, kept as Python: try/except* at 224:5
        rt::exec_python!(
            m,
            line = 222,
            r#"
def groups():
    # `except*` is kept as Python.
    try:
        raise ExceptionGroup("g", [ValueError(1), KeyError(2)])
    except* ValueError as eg:
        print("values", eg.exceptions)
    except* KeyError as eg:
        print("keys", eg.exceptions)
"#
        )?;
        // try_except.py:232, kept as Python: except-as name used outside its handler at 236:5
        rt::exec_python!(
            m,
            line = 232,
            r#"
def name_after_handler():
    # `e` is used after its handler: the function is kept as Python.
    try:
        raise ValueError("x")
    except ValueError as e:
        pass
    try:
        print(e)
    except NameError as err:
        print(type(err).__name__, err)
"#
        )?;
        frame.line(244);
        m.global("print")?.call1((
            m.global("parse")?.call1(("12",))?,
            m.global("parse")?.call1(("x",))?,
        ))?;
        frame.line(245);
        m.global("print")?.call1((
            m.global("parse_or_none")?.call1(("7",))?,
            m.global("parse_or_none")?.call1((py.None(),))?,
            m.global("parse_or_none")?.call1(("y",))?,
        ))?;
        frame.line(246);
        for item in PyList::new(py, ["value", "key", "app", "class", "fine"])?.try_iter()? {
            m.set_global("kind", item?)?;
            frame.line(247);
            m.global("print")?
                .call1((m.global("classify")?.call1((m.global("kind")?,))?,))?;
            py.check_signals()?;
            frame.line(246);
        }
        frame.line(248);
        m.global("chained")?.call0()?;
        frame.line(250);
        match (|| m.global("reraise")?.call0())() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 251, || m.global("ValueError"))? => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(252);
                m.global("print")?
                    .call1(("reraised", m.global("e")?.attr("args")?))?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        frame.line(254);
        match (|| m.global("via_helper")?.call0())() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 255, || m.global("KeyError"))? => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(256);
                m.global("print")?.call1((
                    "helper reraised",
                    m.global("repr")?.call1((m.global("e")?,))?,
                ))?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        frame.line(257);
        m.global("no_active")?.call0()?;
        frame.line(258);
        m.global("bad_handler")?.call0()?;
        frame.line(259);
        m.global("handled_state")?.call0()?;
        frame.line(260);
        m.global("print")?
            .call1((m.global("handler_name_is_deleted")?.call0()?,))?;
        frame.line(261);
        m.global("print")?.call1((m
            .global("bare_except_catches_everything")?
            .call1((m.global("ValueError")?.call0()?,))?,))?;
        frame.line(262);
        m.global("print")?.call1((m
            .global("bare_except_catches_everything")?
            .call1((m.global("KeyboardInterrupt")?.call0()?,))?,))?;
        frame.line(263);
        m.global("print")?.call1((m
            .global("bare_except_catches_everything")?
            .call1((m.global("SystemExit")?.call1((3,))?,))?,))?;
        frame.line(264);
        m.global("exceptions_in_else_and_handlers")?.call0()?;
        frame.line(265);
        m.global("stop_iteration")?.call0()?;
        frame.line(266);
        m.global("generator_into_native")?.call0()?;
        frame.line(267);
        m.global("print")?.call1((m
            .global("loop_with_try")?
            .call1((PyList::new(py, ["1", "x", "4", "y", "9", "100"])?,))?,))?;
        frame.line(269);
        match (|| m.global("risky")?.call1(("app",)))() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 270, || m.global("AppError"))? => {
                let exception = frame.catch(err);
                m.set_global("e", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(271);
                m.global("print")?.call1((
                    "module level",
                    m.global("type")?
                        .call1((m.global("e")?,))?
                        .attr("__name__")?,
                    m.global("e")?.attr("args")?,
                ))?;
                m.unbind_global("e")?;
            }
            Err(err) => return Err(err),
        }
        frame.line(272);
        m.global("print")?.call1((m.globals().contains("e")?,))?;
        frame.line(273);
        m.global("groups")?.call0()?;
        frame.line(274);
        m.global("name_after_handler")?.call0()?;
        Ok(())
    })
}
