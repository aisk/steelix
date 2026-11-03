use pyo3::exceptions::PyIndexError;
use pyo3::exceptions::PyKeyError;
use pyo3::exceptions::PyRuntimeError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// try_finally.py:6
pub fn log<'py>(m: &rt::Module<'py>, args: &Bound<'py, PyTuple>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "log", 6, |frame| {
        frame.line(7);
        rt::call!(rt::builtin(py, "print")?, *args)?;
        Ok(rt::none(py))
    })
}

// try_finally.py:10
pub fn finally_overrides_return<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "finally_overrides_return", 10, |frame| {
        let result: PyResult<Bound<'py, PyAny>> = Ok(rt::str(py, "try"));
        let _finally = frame.finally(result);
        Ok(rt::str(py, "finally"))
    })
}

// try_finally.py:17
pub fn finally_swallows_exception<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "finally_swallows_exception", 17, |frame| {
        frame.line(19);
        let result: PyResult<()> = Err(frame.raise(PyValueError::new_err("lost")));
        let _finally = frame.finally(result);
        Ok(rt::str(m.py(), "swallowed"))
    })
}

// try_finally.py:24
pub fn break_in_finally_swallows<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "break_in_finally_swallows", 24, |frame| {
        let mut i: Option<Bound<'py, PyAny>> = None;
        frame.line(25);
        for item in rt::builtin(m.py(), "range")?.call1((3,))?.try_iter()? {
            i = Some(item?);
            let result = (|| -> PyResult<()> {
                frame.line(27);
                Err(frame.raise(PyKeyError::new_err((rt::local(&i, "i")?.clone().unbind(),))))
            })();
            let _finally = frame.finally(result);
            break;
        }
        frame.line(30);
        Ok(rt::local(&i, "i")?.clone())
    })
}

// try_finally.py:33
pub fn continue_in_finally<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "continue_in_finally", 33, |frame| {
        let mut count = rt::int(py, 0);
        frame.line(35);
        for i in rt::builtin(py, "range")?.call1((n,))?.try_iter()? {
            let i = i?;
            let result = (|| -> PyResult<()> {
                frame.line(37);
                if i.rem(2)?.is_truthy()? {
                    frame.line(38);
                    return Err(frame.raise(PyValueError::new_err((i.clone().unbind(),))));
                }
                frame.line(39);
                count = rt::iadd(&count, 1)?;
                Ok(())
            })();
            let _finally = frame.finally(result);
            py.check_signals()?;
            frame.line(35);
            continue;
        }
        Ok(count)
    })
}

// try_finally.py:45
pub fn return_in_finally_in_loop<'py>(
    m: &rt::Module<'py>,
    items: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "return_in_finally_in_loop", 45, |frame| {
        frame.line(46);
        for item in items.try_iter()? {
            let item = item?;
            let result = (|| -> PyResult<rt::Flow<'py>> {
                frame.line(48);
                if item.eq("stop")? {
                    return Ok(rt::Flow::Return(rt::str(py, "stopped")));
                }
                frame.line(50);
                log(m, &PyTuple::new(py, [&rt::str(py, "item"), &item])?)?;
                Ok(rt::Flow::Next)
            })();
            let finally = frame.finally(result);
            frame.line(52);
            log(m, &PyTuple::new(py, [&rt::str(py, "cleanup"), &item])?)?;
            frame.line(53);
            if item.eq("late")? {
                return Ok(rt::str(py, "late return"));
            }
            if let rt::Flow::Return(value) = finally.resume()? {
                return Ok(value);
            }
            py.check_signals()?;
            frame.line(46);
        }
        Ok(rt::str(py, "end"))
    })
}

// try_finally.py:58
pub fn jumps_run_finally<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "jumps_run_finally", 58, |frame| {
        let out = PyList::empty(py).into_any();
        frame.line(60);
        for i in rt::builtin(py, "range")?.call1((5,))?.try_iter()? {
            let i = i?;
            let result = (|| -> PyResult<rt::Flow<'py>> {
                frame.line(62);
                if i.eq(1)? {
                    return Ok(rt::Flow::Continue);
                }
                frame.line(64);
                if i.eq(3)? {
                    return Ok(rt::Flow::Break);
                }
                frame.line(66);
                out.call_method1("append", (&i,))?;
                Ok(rt::Flow::Next)
            })();
            let finally = frame.finally(result);
            frame.line(68);
            out.getattr("append")?.call1((i.neg()?,))?;
            match finally.resume()? {
                rt::Flow::Break => break,
                rt::Flow::Continue => {
                    py.check_signals()?;
                    frame.line(60);
                    continue;
                }
                _ => {}
            }
            py.check_signals()?;
            frame.line(60);
        }
        Ok(out)
    })
}

// try_finally.py:72
pub fn nested_finally<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "nested_finally", 72, |frame| {
        let result = (|| -> PyResult<Bound<'py, PyAny>> {
            let result = (|| -> PyResult<Bound<'py, PyAny>> {
                frame.line(75);
                log(m, &PyTuple::new(py, ["inner body"])?)?;
                Ok(rt::str(py, "inner"))
            })();
            let finally = frame.finally(result);
            frame.line(78);
            log(m, &PyTuple::new(py, ["inner finally"])?)?;
            finally.resume()
        })();
        let finally = frame.finally(result);
        frame.line(80);
        log(m, &PyTuple::new(py, ["outer finally"])?)?;
        finally.resume()
    })
}

// try_finally.py:83
pub fn exception_in_finally<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "exception_in_finally", 83, |frame| {
        {
            let result: PyResult<()> = {
                frame.line(86);
                let result: PyResult<()> = Err(frame.raise(PyValueError::new_err("first")));
                let _finally = frame.finally(result);
                frame.line(88);
                Err(frame.raise(PyKeyError::new_err("second")))
            };
            match result {
                Ok(()) => unreachable!(),
                Err(err) if err.is_instance_of::<PyKeyError>(py) => {
                    let e = frame.catch(err);
                    let _handling = rt::Handling::new(&e);
                    frame.line(90);
                    log(
                        m,
                        &PyTuple::new(
                            py,
                            [
                                &rt::str(py, "caught"),
                                e.repr()?.as_any(),
                                &rt::str(py, "context"),
                                e.getattr("__context__")?.repr()?.as_any(),
                            ],
                        )?,
                    )?;
                }
                Err(err) => return Err(err),
            }
        }
        Ok(rt::none(py))
    })
}

// try_finally.py:93
pub fn exception_state_in_finally<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "exception_state_in_finally", 93, |frame| {
        match (|| -> PyResult<()> {
            frame.line(96);
            let result: PyResult<()> = Err(frame.raise(PyValueError::new_err("pending")));
            let finally = frame.finally(result);
            frame.line(98);
            log(
                m,
                &PyTuple::new(
                    py,
                    [
                        &rt::str(py, "in finally:"),
                        m.global("sys")?.call_method0("exception")?.repr()?.as_any(),
                    ],
                )?,
            )?;
            finally.resume()?;
            unreachable!();
        })() {
            Ok(()) => unreachable!(),
            Err(err) if err.is_instance_of::<PyValueError>(py) => {}
            Err(err) => return Err(err),
        }
        let result = (|| -> PyResult<()> {
            frame.line(102);
            log(m, &PyTuple::new(py, ["no exception"])?)?;
            Ok(())
        })();
        let finally = frame.finally(result);
        frame.line(104);
        log(
            m,
            &PyTuple::new(
                py,
                [
                    &rt::str(py, "in finally:"),
                    m.global("sys")?.call_method0("exception")?.repr()?.as_any(),
                ],
            )?,
        )?;
        finally.resume()?;
        Ok(rt::none(py))
    })
}

// try_finally.py:107
pub fn finally_after_handler<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "finally_after_handler", 107, |frame| {
        let events = PyList::empty(py).into_any();
        let result = (|| -> PyResult<Bound<'py, PyAny>> {
            match (|| -> PyResult<()> {
                frame.line(110);
                events.call_method1("append", ("body",))?;
                frame.line(111);
                Err(frame.raise(PyIndexError::new_err("x")))
            })() {
                Ok(()) => unreachable!(),
                Err(err) if err.is_instance_of::<PyIndexError>(py) => {
                    let _handling = rt::Handling::new(&frame.catch(err));
                    frame.line(113);
                    events.call_method1("append", ("handler",))?;
                    Ok(events.clone())
                }
                Err(err) => Err(err),
            }
        })();
        let finally = frame.finally(result);
        frame.line(118);
        events.call_method1("append", ("finally",))?;
        finally.resume()
    })
}

// try_finally.py:121
pub fn assigned_in_try<'py>(
    m: &rt::Module<'py>,
    flag: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "assigned_in_try", 121, |frame| {
        let mut value: Option<Bound<'py, PyAny>> = None;
        let result = (|| -> PyResult<()> {
            match (|| -> PyResult<()> {
                value = Some(rt::str(py, "set"));
                frame.line(124);
                if flag.is_truthy()? {
                    frame.line(125);
                    return Err(frame.raise(PyRuntimeError::new_err("boom")));
                }
                Ok(())
            })() {
                Ok(()) => {}
                Err(err) if err.is_instance_of::<PyRuntimeError>(py) => {
                    value = Some(rt::str(py, "handled"))
                }
                Err(err) => return Err(err),
            }
            Ok(())
        })();
        let finally = frame.finally(result);
        frame.line(129);
        log(
            m,
            &PyTuple::new(
                py,
                [&rt::str(py, "finally sees"), rt::local(&value, "value")?],
            )?,
        )?;
        finally.resume()?;
        frame.line(130);
        Ok(rt::local(&value, "value")?.clone())
    })
}

// try_finally.py:133
pub fn while_with_finally<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "while_with_finally", 133, |frame| {
        let mut n = rt::int(py, 0);
        frame.line(135);
        'while_else: {
            while n.lt(10)? {
                let result = (|| -> PyResult<rt::Flow<'py>> {
                    frame.line(137);
                    n = rt::iadd(&n, 1)?;
                    frame.line(138);
                    if n.eq(3)? {
                        return Ok(rt::Flow::Continue);
                    }
                    frame.line(140);
                    if n.eq(5)? {
                        return Ok(rt::Flow::Break);
                    }
                    Ok(rt::Flow::Next)
                })();
                let finally = frame.finally(result);
                frame.line(143);
                log(m, &PyTuple::new(py, [&rt::str(py, "tick"), &n])?)?;
                match finally.resume()? {
                    rt::Flow::Break => break 'while_else,
                    rt::Flow::Continue => {
                        py.check_signals()?;
                        frame.line(135);
                        continue;
                    }
                    _ => {}
                }
                py.check_signals()?;
                frame.line(135);
            }
            frame.line(145);
            log(m, &PyTuple::new(py, ["no break"])?)?;
        }
        Ok(n)
    })
}

/// Executes the body of `try_finally.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        // try/finally: jumps through `finally`, `finally` overriding them, and
        // exceptions raised or swallowed in `finally`.
        frame.line(3);
        m.set_global("sys", rt::import(m, "sys")?)?;
        rt::def!(m, log(*args))?;
        rt::def!(m, finally_overrides_return())?;
        rt::def!(m, finally_swallows_exception())?;
        rt::def!(m, break_in_finally_swallows())?;
        rt::def!(m, continue_in_finally(n))?;
        rt::def!(m, return_in_finally_in_loop(items))?;
        rt::def!(m, jumps_run_finally())?;
        rt::def!(m, nested_finally())?;
        rt::def!(m, exception_in_finally())?;
        rt::def!(m, exception_state_in_finally())?;
        rt::def!(m, finally_after_handler())?;
        rt::def!(m, assigned_in_try(flag))?;
        rt::def!(m, while_with_finally())?;
        frame.line(149);
        rt::builtins::print(py, (finally_overrides_return(m)?,))?;
        frame.line(150);
        rt::builtins::print(py, (finally_swallows_exception(m)?,))?;
        frame.line(151);
        rt::builtins::print(py, (break_in_finally_swallows(m)?,))?;
        frame.line(152);
        rt::builtins::print(py, (continue_in_finally(m, &rt::int(py, 5))?,))?;
        frame.line(153);
        rt::builtins::print(
            py,
            (return_in_finally_in_loop(
                m,
                PyList::new(py, ["a", "late", "b"])?.as_any(),
            )?,),
        )?;
        frame.line(154);
        rt::builtins::print(
            py,
            (return_in_finally_in_loop(
                m,
                PyList::new(py, ["a", "stop", "b"])?.as_any(),
            )?,),
        )?;
        frame.line(155);
        rt::builtins::print(
            py,
            (return_in_finally_in_loop(
                m,
                PyList::new(py, ["a"])?.as_any(),
            )?,),
        )?;
        frame.line(156);
        rt::builtins::print(py, (jumps_run_finally(m)?,))?;
        frame.line(157);
        rt::builtins::print(py, (nested_finally(m)?,))?;
        frame.line(158);
        exception_in_finally(m)?;
        frame.line(159);
        exception_state_in_finally(m)?;
        frame.line(160);
        rt::builtins::print(py, (finally_after_handler(m)?,))?;
        frame.line(161);
        rt::builtins::print(
            py,
            (
                assigned_in_try(m, &rt::bool(py, false))?,
                assigned_in_try(m, &rt::bool(py, true))?,
            ),
        )?;
        frame.line(162);
        rt::builtins::print(py, (while_with_finally(m)?,))?;
        let result = (|| -> PyResult<()> {
            frame.line(164);
            log(m, &PyTuple::new(py, ["module try"])?)?;
            Ok(())
        })();
        let finally = frame.finally(result);
        frame.line(166);
        log(m, &PyTuple::new(py, ["module finally"])?)?;
        finally.resume()?;
        frame.line(167);
        for item in rt::builtin(py, "range")?.call1((3,))?.try_iter()? {
            m.set_global("k", item?)?;
            let result = (|| -> PyResult<rt::Flow<'_>> {
                frame.line(169);
                if m.global("k")?.eq(1)? {
                    return Ok(rt::Flow::Continue);
                }
                frame.line(171);
                log(
                    m,
                    &PyTuple::new(py, [&rt::str(py, "module loop"), &m.global("k")?])?,
                )?;
                Ok(rt::Flow::Next)
            })();
            let finally = frame.finally(result);
            frame.line(173);
            log(
                m,
                &PyTuple::new(py, [&rt::str(py, "module cleanup"), &m.global("k")?])?,
            )?;
            if let rt::Flow::Continue = finally.resume()? {
                py.check_signals()?;
                frame.line(167);
                continue;
            }
            py.check_signals()?;
            frame.line(167);
        }
        Ok(())
    })
}
