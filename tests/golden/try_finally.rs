use pyo3::exceptions::PyIndexError;
use pyo3::exceptions::PyKeyError;
use pyo3::exceptions::PyRuntimeError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// try_finally.py:6
#[rt::frame(m, "log", 6)]
pub fn log<'py>(m: &rt::Module<'py>, args: &Bound<'py, PyTuple>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::call!(rt::builtin(py, "print")?, *args)?;
    Ok(rt::none(py))
}

// try_finally.py:10
#[rt::frame(m, "finally_overrides_return", 10, frame)]
pub fn finally_overrides_return<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let result: PyResult<Bound<'py, PyAny>> = Ok(rt::str(py, "try"));
    let _finally = frame.finally(result);
    Ok(rt::str(py, "finally"))
}

// try_finally.py:17
#[rt::frame(m, "finally_swallows_exception", 17, frame)]
pub fn finally_swallows_exception<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let result: PyResult<()> = Err(frame.raise(PyValueError::new_err("lost")));
    let _finally = frame.finally(result);
    Ok(rt::str(m.py(), "swallowed"))
}

// try_finally.py:24
#[rt::frame(m, "break_in_finally_swallows", 24, frame)]
pub fn break_in_finally_swallows<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let mut i: Option<Bound<'py, PyAny>> = None;
    for item in rt::iter(&rt::builtins::range(m.py(), (3,))?)? {
        i = Some(item?);
        let result = (|| -> PyResult<()> {
            Err(frame.raise(PyKeyError::new_err((rt::local(&i, "i")?.clone().unbind(),))))
        })();
        let _finally = frame.finally(result);
        break;
    }
    Ok(rt::local(&i, "i")?.clone())
}

// try_finally.py:33
#[rt::frame(m, "continue_in_finally", 33, frame)]
pub fn continue_in_finally<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut count = rt::int(py, 0);
    for i in rt::iter(&rt::builtins::range(py, (n,))?)? {
        let i = i?;
        let result = (|| -> PyResult<()> {
            if i.rem(2)?.is_truthy()? {
                return Err(frame.raise(PyValueError::new_err((i.clone().unbind(),))));
            }
            count = rt::iadd(&count, 1)?;
            Ok(())
        })();
        let _finally = frame.finally(result);
        continue;
    }
    Ok(count)
}

// try_finally.py:45
#[rt::frame(m, "return_in_finally_in_loop", 45, frame)]
pub fn return_in_finally_in_loop<'py>(
    m: &rt::Module<'py>,
    items: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    for item in rt::iter(items)? {
        let item = item?;
        let result = (|| -> PyResult<rt::Flow<'py>> {
            if item.eq("stop")? {
                return Ok(rt::Flow::Return(rt::str(py, "stopped")));
            }
            log(m, &PyTuple::new(py, [&rt::str(py, "item"), &item])?)?;
            Ok(rt::Flow::Next)
        })();
        let finally = frame.finally(result);
        log(m, &PyTuple::new(py, [&rt::str(py, "cleanup"), &item])?)?;
        if item.eq("late")? {
            return Ok(rt::str(py, "late return"));
        }
        if let rt::Flow::Return(value) = finally.resume()? {
            return Ok(value);
        }
    }
    Ok(rt::str(py, "end"))
}

// try_finally.py:58
#[rt::frame(m, "jumps_run_finally", 58, frame)]
pub fn jumps_run_finally<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let out = PyList::empty(py).into_any();
    for i in rt::iter(&rt::builtins::range(py, (5,))?)? {
        let i = i?;
        let result = (|| -> PyResult<rt::Flow<'py>> {
            if i.eq(1)? {
                return Ok(rt::Flow::Continue);
            }
            if i.eq(3)? {
                return Ok(rt::Flow::Break);
            }
            out.method1("append", (&i,))?;
            Ok(rt::Flow::Next)
        })();
        let finally = frame.finally(result);
        out.attr("append")?.call1((i.neg()?,))?;
        match finally.resume()? {
            rt::Flow::Break => break,
            rt::Flow::Continue => continue,
            _ => {}
        }
    }
    Ok(out)
}

// try_finally.py:72
#[rt::frame(m, "nested_finally", 72, frame)]
pub fn nested_finally<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let result = (|| -> PyResult<Bound<'py, PyAny>> {
        let result = (|| -> PyResult<Bound<'py, PyAny>> {
            log(m, &PyTuple::new(py, ["inner body"])?)?;
            Ok(rt::str(py, "inner"))
        })();
        let finally = frame.finally(result);
        log(m, &PyTuple::new(py, ["inner finally"])?)?;
        finally.resume()
    })();
    let finally = frame.finally(result);
    log(m, &PyTuple::new(py, ["outer finally"])?)?;
    finally.resume()
}

// try_finally.py:83
#[rt::frame(m, "exception_in_finally", 83, frame)]
pub fn exception_in_finally<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    {
        let result: PyResult<()> = {
            let result: PyResult<()> = Err(frame.raise(PyValueError::new_err("first")));
            let _finally = frame.finally(result);
            Err(frame.raise(PyKeyError::new_err("second")))
        };
        match result {
            Ok(()) => unreachable!(),
            Err(err) if err.is_instance_of::<PyKeyError>(py) => {
                let e = frame.catch(err);
                log(
                    m,
                    &PyTuple::new(
                        py,
                        [
                            &rt::str(py, "caught"),
                            e.repr()?.as_any(),
                            &rt::str(py, "context"),
                            e.attr("__context__")?.repr()?.as_any(),
                        ],
                    )?,
                )?;
            }
            Err(err) => return Err(err),
        }
    }
    Ok(rt::none(py))
}

// try_finally.py:93
#[rt::frame(m, "exception_state_in_finally", 93, frame)]
pub fn exception_state_in_finally<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    match (|| -> PyResult<()> {
        let result: PyResult<()> = Err(frame.raise(PyValueError::new_err("pending")));
        let finally = frame.finally(result);
        log(
            m,
            &PyTuple::new(
                py,
                [
                    &rt::str(py, "in finally:"),
                    m.global("sys")?.method0("exception")?.repr()?.as_any(),
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
        log(m, &PyTuple::new(py, ["no exception"])?)?;
        Ok(())
    })();
    let finally = frame.finally(result);
    log(
        m,
        &PyTuple::new(
            py,
            [
                &rt::str(py, "in finally:"),
                m.global("sys")?.method0("exception")?.repr()?.as_any(),
            ],
        )?,
    )?;
    finally.resume()?;
    Ok(rt::none(py))
}

// try_finally.py:107
#[rt::frame(m, "finally_after_handler", 107, frame)]
pub fn finally_after_handler<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let events = PyList::empty(py).into_any();
    let result = (|| -> PyResult<Bound<'py, PyAny>> {
        match (|| -> PyResult<()> {
            events.method1("append", ("body",))?;
            Err(frame.raise(PyIndexError::new_err("x")))
        })() {
            Ok(()) => unreachable!(),
            Err(err) if err.is_instance_of::<PyIndexError>(py) => {
                let _caught = frame.catch(err);
                events.method1("append", ("handler",))?;
                Ok(events.clone())
            }
            Err(err) => Err(err),
        }
    })();
    let finally = frame.finally(result);
    events.method1("append", ("finally",))?;
    finally.resume()
}

// try_finally.py:121
#[rt::frame(m, "assigned_in_try", 121, frame)]
pub fn assigned_in_try<'py>(
    m: &rt::Module<'py>,
    flag: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut value: Option<Bound<'py, PyAny>> = None;
    let result = (|| -> PyResult<()> {
        match (|| -> PyResult<()> {
            value = Some(rt::str(py, "set"));
            if flag.is_truthy()? {
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
    log(
        m,
        &PyTuple::new(
            py,
            [&rt::str(py, "finally sees"), rt::local(&value, "value")?],
        )?,
    )?;
    finally.resume()?;
    Ok(rt::local(&value, "value")?.clone())
}

// try_finally.py:133
#[rt::frame(m, "while_with_finally", 133, frame)]
pub fn while_with_finally<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut n = rt::int(py, 0);
    'while_else: {
        while n.lt(10)? {
            let result = (|| -> PyResult<rt::Flow<'py>> {
                n = rt::iadd(&n, 1)?;
                if n.eq(3)? {
                    return Ok(rt::Flow::Continue);
                }
                if n.eq(5)? {
                    return Ok(rt::Flow::Break);
                }
                Ok(rt::Flow::Next)
            })();
            let finally = frame.finally(result);
            log(m, &PyTuple::new(py, [&rt::str(py, "tick"), &n])?)?;
            match finally.resume()? {
                rt::Flow::Break => break 'while_else,
                rt::Flow::Continue => {
                    py.check_signals()?;
                    continue;
                }
                _ => {}
            }
            py.check_signals()?;
        }
        log(m, &PyTuple::new(py, ["no break"])?)?;
    }
    Ok(n)
}

/// Executes the body of `try_finally.py` in the module `m`.
#[rt::frame(m, "<module>", 1, frame)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    // try/finally: jumps through `finally`, `finally` overriding them, and
    // exceptions raised or swallowed in `finally`.
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
    rt::builtins::print(py, (finally_overrides_return(m)?,))?;
    rt::builtins::print(py, (finally_swallows_exception(m)?,))?;
    rt::builtins::print(py, (break_in_finally_swallows(m)?,))?;
    rt::builtins::print(py, (continue_in_finally(m, &rt::int(py, 5))?,))?;
    rt::builtins::print(
        py,
        (return_in_finally_in_loop(
            m,
            PyList::new(py, ["a", "late", "b"])?.as_any(),
        )?,),
    )?;
    rt::builtins::print(
        py,
        (return_in_finally_in_loop(
            m,
            PyList::new(py, ["a", "stop", "b"])?.as_any(),
        )?,),
    )?;
    rt::builtins::print(
        py,
        (return_in_finally_in_loop(
            m,
            PyList::new(py, ["a"])?.as_any(),
        )?,),
    )?;
    rt::builtins::print(py, (jumps_run_finally(m)?,))?;
    rt::builtins::print(py, (nested_finally(m)?,))?;
    exception_in_finally(m)?;
    exception_state_in_finally(m)?;
    rt::builtins::print(py, (finally_after_handler(m)?,))?;
    rt::builtins::print(
        py,
        (
            assigned_in_try(m, &rt::bool(py, false))?,
            assigned_in_try(m, &rt::bool(py, true))?,
        ),
    )?;
    rt::builtins::print(py, (while_with_finally(m)?,))?;
    let result = (|| -> PyResult<()> {
        log(m, &PyTuple::new(py, ["module try"])?)?;
        Ok(())
    })();
    let finally = frame.finally(result);
    log(m, &PyTuple::new(py, ["module finally"])?)?;
    finally.resume()?;
    for item in rt::iter(&rt::builtins::range(py, (3,))?)? {
        m.set_global("k", item?)?;
        let result = (|| -> PyResult<rt::Flow<'_>> {
            if m.global("k")?.eq(1)? {
                return Ok(rt::Flow::Continue);
            }
            log(
                m,
                &PyTuple::new(py, [&rt::str(py, "module loop"), &m.global("k")?])?,
            )?;
            Ok(rt::Flow::Next)
        })();
        let finally = frame.finally(result);
        log(
            m,
            &PyTuple::new(py, [&rt::str(py, "module cleanup"), &m.global("k")?])?,
        )?;
        if let rt::Flow::Continue = finally.resume()? {
            continue;
        }
    }
    Ok(())
}
