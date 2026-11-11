use pyo3::exceptions::PyAssertionError;
use pyo3::exceptions::PyTypeError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use pyo3::types::PySet;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// unit_functions.py:9
#[rt::frame(m, "note", 9)]
pub fn note<'py>(m: &rt::Module<'py>, label: &Bound<'py, PyAny>) -> PyResult<()> {
    m.global("log")?.method1("append", (label,))?;
    rt::builtins::print(m.py(), ("note", label))?;
    Ok(())
}

// unit_functions.py:14
#[rt::frame(m, "proc", 14)]
pub fn proc<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<()> {
    rt::builtins::print(m.py(), ("proc", x))?;
    Ok(())
}

// unit_functions.py:18
#[rt::frame(m, "early", 18)]
pub fn early<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<()> {
    let py = m.py();
    if x.lt(0)? {
        rt::builtins::print(py, ("negative",))?;
        return Ok(());
    }
    if x.eq(0)? {
        return Ok(());
    }
    rt::builtins::print(py, ("positive", x))?;
    Ok(())
}

// unit_functions.py:27
pub fn explicit_none<'py>(m: &rt::Module<'py>) -> PyResult<()> {
    Ok(())
}

// unit_functions.py:31
pub fn empty<'py>(m: &rt::Module<'py>) -> PyResult<()> {
    Ok(())
}

// unit_functions.py:35
/// Does nothing.
pub fn only_docstring<'py>(m: &rt::Module<'py>) -> PyResult<()> {
    Ok(())
}

// unit_functions.py:39
#[rt::frame(m, "mixed", 39)]
pub fn mixed<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    if x.is_truthy()? {
        return Ok(x.clone());
    }
    Ok(rt::none(m.py()))
}

// unit_functions.py:45
#[rt::frame(m, "tail_call", 45)]
pub fn tail_call<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    proc(m, x)?;
    Ok(rt::none(m.py()))
}

// unit_functions.py:49
#[rt::frame(m, "always_raises", 49, frame)]
pub fn always_raises<'py>(m: &rt::Module<'py>, message: &Bound<'py, PyAny>) -> PyResult<()> {
    Err(frame.raise(PyValueError::new_err((message.clone().unbind(),))))
}

// unit_functions.py:53
#[rt::frame(m, "loop_return", 53)]
pub fn loop_return<'py>(
    m: &rt::Module<'py>,
    items: &Bound<'py, PyAny>,
    stop: &Bound<'py, PyAny>,
) -> PyResult<()> {
    let py = m.py();
    for item in rt::iter(items)? {
        let item = item?;
        if item.eq(stop)? {
            return Ok(());
        }
        rt::builtins::print(py, ("item", &item))?;
    }
    rt::builtins::print(py, ("no stop",))?;
    Ok(())
}

// unit_functions.py:62
#[rt::frame(m, "while_return", 62)]
pub fn while_return<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<()> {
    let py = m.py();
    let mut n = n.clone();
    loop {
        n = rt::isub(&n, 1)?;
        if n.lt(0)? {
            return Ok(());
        }
        rt::builtins::print(py, ("n", &n))?;
        py.check_signals()?;
    }
}

// unit_functions.py:70
#[rt::frame(m, "match_return", 70)]
pub fn match_return<'py>(m: &rt::Module<'py>, value: &Bound<'py, PyAny>) -> PyResult<()> {
    let py = m.py();
    if value.eq(0)? {
        return Ok(());
    } else if let Some([a, b]) = rt::match_seq(value)? {
        rt::builtins::print(py, ("pair", &a, &b))?;
        return Ok(());
    } else {
        rt::builtins::print(py, ("other", value))?;
    }
    Ok(())
}

// unit_functions.py:81
#[rt::frame(m, "with_defaults", 81)]
pub fn with_defaults<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
    rest: &Bound<'py, PyTuple>,
    c: &Bound<'py, PyAny>,
    items: &Bound<'py, PyAny>,
    options: &Bound<'py, PyDict>,
) -> PyResult<()> {
    rt::builtins::print(m.py(), ("with_defaults", a, b, rest, c, items, options))?;
    Ok(())
}

// unit_functions.py:85
#[rt::frame(m, "countdown", 85, recursive)]
pub fn countdown<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<()> {
    let py = m.py();
    if n.eq(0)? {
        rt::builtins::print(py, ("liftoff",))?;
        return Ok(());
    }
    rt::builtins::print(py, (n,))?;
    countdown(m, &n.sub(1)?)?;
    Ok(())
}

// unit_functions.py:93
#[rt::frame(m, "value", 93)]
pub fn value<'py>(m: &rt::Module<'py>, label: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::builtins::print(m.py(), ("value", label))?;
    Ok(label.clone())
}

// unit_functions.py:231
#[rt::frame(m, "uses", 231)]
pub fn uses<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    proc(m, &rt::str(py, "statement"))?;
    proc(m, &rt::str(py, "local"))?;
    let mut r = rt::none(py);
    rt::builtins::print(py, (&r,))?;
    {
        proc(m, &rt::str(py, "a"))?;
        let value2 = value(m, &rt::str(py, "b"))?;
        proc(m, &rt::str(py, "c"))?;
        r = PyList::new(py, [&rt::none(py), &value2, &rt::none(py)])?.into_any();
    }
    rt::builtins::print(py, (&r,))?;
    proc(m, &rt::str(py, "condition"))?;
    if rt::none(py).is_none() {
        rt::builtins::print(py, ("is None",))?;
    }
    early(m, &rt::int(py, 5))?;
    Ok(rt::none(py))
}

// unit_functions.py:245
#[rt::frame(m, "returns_unit_call", 245)]
pub fn returns_unit_call<'py>(
    m: &rt::Module<'py>,
    flag: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    if flag.is_truthy()? {
        proc(m, &rt::str(py, "returned"))?;
        return Ok(rt::none(py));
    }
    early(m, &rt::int(py, -1))?;
    Ok(rt::none(py))
}

// unit_functions.py:254
#[rt::frame(m, "unit_calling_units", 254)]
pub fn unit_calling_units<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<()> {
    proc(m, x)?;
    early(m, x)?;
    empty(m)?;
    Ok(())
}

// unit_functions.py:263
#[rt::frame(m, "order", 263)]
pub fn order<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let value2 = value(m, &rt::int(py, 1))?;
    proc(m, x)?;
    let value3 = PyList::new(py, [&value2, &rt::none(py), &value(m, &rt::int(py, 2))?])?;
    proc(m, &rt::str(py, "cond"))?;
    Ok(PyTuple::new(
        py,
        [
            value3.as_any(),
            &(if false {
                value(m, &rt::int(py, 3))?
            } else {
                value(m, &rt::int(py, 4))?
            }),
        ],
    )?
    .into_any())
}

// unit_functions.py:286
#[rt::frame(m, "try_finally", 286, frame)]
pub fn try_finally<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<()> {
    let py = m.py();
    let result = (|| -> PyResult<rt::Jump> {
        if x.eq(0)? {
            return Ok(rt::Jump::Return);
        }
        if x.eq(1)? {
            return Ok(rt::Jump::Return);
        }
        rt::builtins::print(py, ("try", x))?;
        Ok(rt::Jump::Next)
    })();
    let finally = frame.finally(result);
    rt::builtins::print(py, ("finally", x))?;
    if let rt::Jump::Return = finally.resume()? {
        return Ok(());
    }
    rt::builtins::print(py, ("after", x))?;
    Ok(())
}

// unit_functions.py:298
#[rt::frame(m, "try_except", 298, frame)]
pub fn try_except<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<()> {
    let py = m.py();
    match (|| -> PyResult<rt::Jump> {
        if x.eq(0)? {
            return Ok(rt::Jump::Return);
        }
        always_raises(m, &rt::str(py, "in try"))?;
        Ok(rt::Jump::Next)
    })() {
        Ok(rt::Jump::Return) => return Ok(()),
        Ok(_) => {
            rt::builtins::print(py, ("else",))?;
        }
        Err(err) if err.is_instance_of::<PyValueError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("caught", &e))?;
            if x.eq(1)? {
                return Ok(());
            }
        }
        Err(err) => return Err(err),
    }
    rt::builtins::print(py, ("end", x))?;
    Ok(())
}

// unit_functions.py:312
#[rt::frame(m, "try_always_returns", 312, frame)]
pub fn try_always_returns<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<()> {
    let py = m.py();
    let result = (|| -> PyResult<()> {
        rt::builtins::print(py, ("body", x))?;
        Ok(())
    })();
    let finally = frame.finally(result);
    rt::builtins::print(py, ("cleanup", x))?;
    finally.resume()
}

// unit_functions.py:320
#[rt::frame(m, "try_except_returns", 320, frame)]
pub fn try_except_returns<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<()> {
    let py = m.py();
    match (|| -> PyResult<()> {
        rt::builtins::print(py, ("body", rt::builtins::int(py, (x,))?))?;
        Ok(())
    })() {
        Ok(()) => return Ok(()),
        Err(err) if err.is_instance_of::<PyValueError>(py) => {
            let _caught = frame.catch(err);
            rt::builtins::print(py, ("not a number", x))?;
        }
        Err(err) => return Err(err),
    }
    Ok(())
}

// unit_functions.py:328
pub fn try_return_none<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<()> {
    Ok(())
}

// unit_functions.py:335
#[rt::frame(m, "try_call", 335, frame)]
pub fn try_call<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    match proc(m, x) {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyValueError>(py) => {
            let _caught = frame.catch(err);
            rt::builtins::print(py, ("unreachable",))?;
        }
        Err(err) => return Err(err),
    }
    match always_raises(m, x) {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyValueError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("caught", &e))?;
        }
        Err(err) => return Err(err),
    }
    match (|| always_raises(m, &value(m, x)?))() {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyValueError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("caught", &e))?;
        }
        Err(err) => return Err(err),
    }
    let r = match (|| -> PyResult<Bound<'py, PyAny>> {
        proc(m, x)?;
        Ok(rt::none(py))
    })() {
        Ok(value2) => value2,
        Err(err) if err.is_instance_of::<PyValueError>(py) => rt::str(py, "unreachable"),
        Err(err) => return Err(err),
    };
    match (|| -> PyResult<Bound<'py, PyAny>> {
        proc(m, &r)?;
        Ok(rt::none(py))
    })() {
        Ok(value2) => return Ok(value2),
        Err(err) if err.is_instance_of::<PyValueError>(py) => {
            let _caught = frame.catch(err);
            rt::builtins::print(py, ("unreachable",))?;
        }
        Err(err) => return Err(err),
    }
    Ok(rt::none(py))
}

// unit_functions.py:358
#[rt::frame(m, "try_call_value", 358, frame)]
pub fn try_call_value<'py>(
    m: &rt::Module<'py>,
    x: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    match (|| -> PyResult<Bound<'py, PyAny>> {
        always_raises(m, x)?;
        Ok(rt::none(py))
    })() {
        Ok(value2) => Ok(value2),
        Err(err) if err.is_instance_of::<PyValueError>(py) => {
            let e = frame.catch(err);
            Ok(e.str()?.into_any())
        }
        Err(err) => Err(err),
    }
}

// unit_functions.py:365
#[rt::frame(m, "with_return", 365, frame)]
pub fn with_return<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<()> {
    let py = m.py();
    if let rt::Jump::Return = frame.with(&m.global("Manager")?.call1(("a",))?, |_| {
        if x.eq(0)? {
            return Ok(rt::Jump::Return);
        }
        rt::builtins::print(py, ("with", x))?;
        Ok(rt::Jump::Next)
    })? {
        return Ok(());
    }
    rt::builtins::print(py, ("after with", x))?;
    Ok(())
}

// unit_functions.py:373
#[rt::frame(m, "with_always_returns", 373, frame)]
pub fn with_always_returns<'py>(m: &rt::Module<'py>) -> PyResult<()> {
    if let rt::Jump::Return = frame.with(&m.global("Manager")?.call1(("b",))?, |_| {
        frame.with(&m.global("Manager")?.call1(("c",))?, |_| {
            Ok(rt::Jump::Return)
        })
    })? {
        return Ok(());
    }
    Ok(())
}

// unit_functions.py:378
#[rt::frame(m, "with_suppressed", 378, frame)]
pub fn with_suppressed<'py>(m: &rt::Module<'py>) -> PyResult<()> {
    let py = m.py();
    frame.with(
        &rt::call!(m.global("Manager")?, "d", suppress = true)?,
        |_| {
            always_raises(m, &rt::str(py, "suppressed"))?;
            Ok(())
        },
    )?;
    rt::builtins::print(py, ("after suppressed",))?;
    Ok(())
}

// unit_functions.py:384
#[rt::frame(m, "with_loop", 384, frame)]
pub fn with_loop<'py>(m: &rt::Module<'py>, items: &Bound<'py, PyAny>) -> PyResult<()> {
    let py = m.py();
    for item in rt::iter(items)? {
        let item = item?;
        match frame.with(&m.global("Manager")?.call1((&item,))?, |_| {
            if item.eq("skip")? {
                return Ok(rt::Jump::Continue);
            }
            if item.eq("stop")? {
                return Ok(rt::Jump::Break);
            }
            if item.eq("return")? {
                return Ok(rt::Jump::Return);
            }
            rt::builtins::print(py, ("body", &item))?;
            Ok(rt::Jump::Next)
        })? {
            rt::Jump::Return => return Ok(()),
            rt::Jump::Break => break,
            rt::Jump::Continue => continue,
            rt::Jump::Next => {}
        }
    }
    rt::builtins::print(py, ("loop done",))?;
    Ok(())
}

// unit_functions.py:397
#[rt::frame(m, "nested_regions", 397, frame)]
pub fn nested_regions<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<()> {
    let py = m.py();
    let result = (|| -> PyResult<rt::Jump> {
        match (|| -> PyResult<rt::Jump> {
            if let rt::Jump::Return =
                frame.with(&m.global("Manager")?.call1(("outer",))?, |_| {
                    let result = (|| -> PyResult<rt::Jump> {
                        if x.eq(0)? {
                            return Ok(rt::Jump::Return);
                        }
                        if x.eq(1)? {
                            always_raises(m, &rt::str(py, "inner"))?;
                        }
                        Ok(rt::Jump::Next)
                    })();
                    let finally = frame.finally(result);
                    rt::builtins::print(py, ("inner finally",))?;
                    if let rt::Jump::Return = finally.resume()? {
                        return Ok(rt::Jump::Return);
                    }
                    rt::builtins::print(py, ("in with",))?;
                    Ok(rt::Jump::Next)
                })?
            {
                return Ok(rt::Jump::Return);
            }
            Ok(rt::Jump::Next)
        })() {
            Ok(rt::Jump::Return) => return Ok(rt::Jump::Return),
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyValueError>(py) => {
                let e = frame.catch(err);
                rt::builtins::print(py, ("caught", &e))?;
                return Ok(rt::Jump::Return);
            }
            Err(err) => return Err(err),
        }
        Ok(rt::Jump::Next)
    })();
    let finally = frame.finally(result);
    rt::builtins::print(py, ("outer finally",))?;
    if let rt::Jump::Return = finally.resume()? {
        return Ok(());
    }
    rt::builtins::print(py, ("end", x))?;
    Ok(())
}

// unit_functions.py:416
#[rt::frame(m, "finally_returns", 416, frame)]
pub fn finally_returns<'py>(m: &rt::Module<'py>) -> PyResult<()> {
    let result = (|| -> PyResult<()> {
        always_raises(m, &rt::str(m.py(), "dropped"))?;
        Ok(())
    })();
    let _finally = frame.finally(result);
    Ok(())
}

// unit_functions.py:423
#[rt::frame(m, "loop_try", 423, frame)]
pub fn loop_try<'py>(m: &rt::Module<'py>, items: &Bound<'py, PyAny>) -> PyResult<()> {
    let py = m.py();
    for item in rt::iter(items)? {
        let item = item?;
        let result = (|| -> PyResult<rt::Jump> {
            if item.eq(1)? {
                return Ok(rt::Jump::Continue);
            }
            if item.eq(2)? {
                return Ok(rt::Jump::Break);
            }
            if item.eq(3)? {
                return Ok(rt::Jump::Return);
            }
            rt::builtins::print(py, ("item", &item))?;
            Ok(rt::Jump::Next)
        })();
        let finally = frame.finally(result);
        rt::builtins::print(py, ("finally", &item))?;
        match finally.resume()? {
            rt::Jump::Return => return Ok(()),
            rt::Jump::Break => break,
            rt::Jump::Continue => continue,
            rt::Jump::Next => {}
        }
    }
    rt::builtins::print(py, ("done",))?;
    Ok(())
}

// unit_functions.py:458
#[rt::frame(m, "counter", 458, frame)]
pub fn counter<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let count = rt::Cell::new(py, "count", 0)?;
    // unit_functions.py:461
    #[rt::frame(m, "counter.<locals>.bump", 461)]
    fn bump<'py>(
        m: &rt::Module<'py>,
        count: &rt::Cell<'py>,
        step: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        count.set(rt::iadd(&count.get()?, step)?)?;
        Ok(())
    }
    let bump = rt::function!(frame, bump[&count](step = 1))?;
    // unit_functions.py:465
    #[rt::frame(m, "counter.<locals>.reset", 465)]
    fn reset<'py>(m: &rt::Module<'py>, count: &rt::Cell<'py>) -> PyResult<()> {
        if count.get()?.eq(0)? {
            return Ok(());
        }
        count.set(0)?;
        Ok(())
    }
    let reset = rt::function!(frame, reset[&count]())?;
    // unit_functions.py:471
    #[rt::frame(m, "counter.<locals>.show", 471)]
    fn show<'py>(m: &rt::Module<'py>, count: &rt::Cell<'py>) -> PyResult<()> {
        rt::builtins::print(m.py(), ("count", count.get()?))?;
        Ok(())
    }
    let show = rt::function!(frame, show[&count]())?;
    rt::builtins::print(py, (bump.call0()?, bump.call1((5,))?))?;
    show.call0()?;
    rt::builtins::print(py, (reset.call0()?, reset.call0()?))?;
    show.call0()?;
    Ok(PyTuple::new(py, [&bump, &show])?.into_any())
}

// unit_functions.py:486
#[rt::frame(m, "recursive_nested", 486, frame)]
pub fn recursive_nested<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let down = rt::Cell::empty(m.py(), "down")?;
    // unit_functions.py:487
    #[rt::frame(m, "recursive_nested.<locals>.down", 487)]
    fn down_<'py>(
        m: &rt::Module<'py>,
        down: &rt::Cell<'py>,
        k: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        if k.eq(0)? {
            return Ok(());
        }
        rt::builtins::print(m.py(), ("down", k))?;
        down.get()?.call1((k.sub(1)?,))?;
        Ok(())
    }
    down.set(rt::function!(frame, down_ as "down"[&down](k))?)?;
    down.get()?.call1((n,))
}

// unit_functions.py:574
#[rt::frame(m, "traced", 574, frame)]
pub fn traced<'py>(m: &rt::Module<'py>, r#fn: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    // unit_functions.py:575
    #[rt::frame(m, "traced.<locals>.wrapper", 575)]
    fn wrapper<'py>(
        m: &rt::Module<'py>,
        r#fn: &Bound<'py, PyAny>,
        args: &Bound<'py, PyTuple>,
        kwargs: &Bound<'py, PyDict>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let result = rt::call!(r#fn, *args, **kwargs)?;
        rt::builtins::print(
            m.py(),
            ("traced", r#fn.attr("__name__")?, args, kwargs, &result),
        )?;
        Ok(result)
    }
    let wrapper = rt::function!(
        frame,
        @(m.global("functools")?.method1("wraps", (r#fn,))?)
        wrapper[r#fn](*args, **kwargs)
    )?;
    Ok(wrapper)
}

// unit_functions.py:584
#[rt::frame(m, "announce", 584)]
pub fn announce<'py>(m: &rt::Module<'py>, r#fn: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::builtins::print(
        m.py(),
        ("announce", r#fn.attr("__name__")?, r#fn.call1((0,))?),
    )?;
    Ok(r#fn.clone())
}

// unit_functions.py:589
#[rt::frame(m, "decorated", 589)]
pub fn decorated<'py>(
    m: &rt::Module<'py>,
    x: &Bound<'py, PyAny>,
    y: &Bound<'py, PyAny>,
) -> PyResult<()> {
    rt::builtins::print(m.py(), ("decorated", x, y))?;
    Ok(())
}

// unit_functions.py:594
#[rt::frame(m, "announced", 594)]
pub fn announced<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<()> {
    if x.is_truthy()? {
        return Ok(());
    }
    rt::builtins::print(m.py(), ("announced", x))?;
    Ok(())
}

// unit_functions.py:601
#[rt::frame(m, "empty_key", 601)]
pub fn empty_key<'py>(m: &rt::Module<'py>, item: &Bound<'py, PyAny>) -> PyResult<()> {
    m.global("log")?.method1("append", (item,))?;
    Ok(())
}

// unit_functions.py:605
#[rt::frame(m, "cached", 605)]
pub fn cached<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<()> {
    rt::builtins::print(m.py(), ("computing", x))?;
    Ok(())
}

// unit_functions.py:623
#[rt::frame(m, "fails_late", 623)]
pub fn fails_late<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<()> {
    let py = m.py();
    proc(m, x)?;
    if x.is_truthy()? {
        always_raises(m, &rt::str(py, "uncaught in a unit function"))?;
    }
    rt::builtins::print(py, ("unreachable",))?;
    Ok(())
}

// unit_functions.py:630
#[rt::frame(m, "outer", 630)]
pub fn outer<'py>(m: &rt::Module<'py>) -> PyResult<()> {
    let py = m.py();
    fails_late(m, &rt::int(py, 1))?;
    let r = rt::none(py);
    rt::builtins::print(py, (&r,))?;
    Ok(())
}

pub mod manager {
    use super::*;

    // unit_functions.py:272
    /// The body of `class Manager`, run in its namespace `ns`.
    #[rt::frame(ns, "Manager", 272)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __init__(slf as "self", name, suppress = false))?;
        rt::def!(ns, __enter__(slf as "self"))?;
        rt::def!(ns, __exit__(slf as "self", kind, exc, tb))?;
        Ok(())
    }

    // unit_functions.py:273
    #[rt::frame(m, "Manager.__init__", 273)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
        suppress: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        slf.set_attr("name", name)?;
        slf.set_attr("suppress", suppress)?;
        Ok(())
    }

    // unit_functions.py:277
    #[rt::frame(m, "Manager.__enter__", 277)]
    pub fn __enter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::builtins::print(m.py(), ("enter", slf.attr("name")?))?;
        Ok(slf.clone())
    }

    // unit_functions.py:281
    #[rt::frame(m, "Manager.__exit__", 281)]
    pub fn __exit__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        kind: &Bound<'py, PyAny>,
        exc: &Bound<'py, PyAny>,
        tb: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::builtins::print(
            py,
            (
                "exit",
                slf.attr("name")?,
                if kind.is_truthy()? {
                    kind.attr("__name__")?
                } else {
                    rt::none(py)
                },
            ),
        )?;
        slf.attr("suppress")
    }
}

pub mod widget {
    use super::*;

    // unit_functions.py:504
    /// The body of `class Widget`, run in its namespace `ns`.
    #[rt::frame(ns, "Widget", 504)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        ns.set("instances", 0)?;
        rt::def!(ns, __init__(slf as "self", name))?;
        rt::def!(ns, rename(slf as "self", name))?;
        rt::def!(ns, @staticmethod describe(text))?;
        rt::def!(ns, @classmethod register(cls))?;
        rt::def!(ns, @property size(slf as "self"))?;
        rt::def!(ns, @size.setter size_ as "size"(slf as "self", value))?;
        rt::def!(ns, @property nothing(slf as "self"))?;
        rt::def!(ns, __setitem__(slf as "self", key, value))?;
        rt::def!(ns, __call__(slf as "self"))?;
        rt::def!(ns, __del__(slf as "self"))?;
        Ok(())
    }

    // unit_functions.py:507
    #[rt::frame(m, "Widget.__init__", 507)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        slf.set_attr("name", name)?;
        slf.set_attr("_size", 0)?;
        m.global("Widget")?.method0("register")?;
        Ok(())
    }

    // unit_functions.py:512
    #[rt::frame(m, "Widget.rename", 512)]
    pub fn rename<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        if name.eq(slf.attr("name")?)? {
            return Ok(());
        }
        slf.set_attr("name", name)?;
        Ok(())
    }

    // unit_functions.py:517
    #[rt::frame(m, "Widget.describe", 517)]
    pub fn describe<'py>(m: &rt::Module<'py>, text: &Bound<'py, PyAny>) -> PyResult<()> {
        rt::builtins::print(m.py(), ("describe", text))?;
        Ok(())
    }

    // unit_functions.py:521
    #[rt::frame(m, "Widget.register", 521)]
    pub fn register<'py>(m: &rt::Module<'py>, cls: &Bound<'py, PyAny>) -> PyResult<()> {
        cls.set_attr("instances", rt::iadd(&cls.attr("instances")?, 1)?)?;
        Ok(())
    }

    // unit_functions.py:525
    #[rt::frame(m, "Widget.size", 525)]
    pub fn size<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        slf.attr("_size")
    }

    // unit_functions.py:529
    #[rt::frame(m, "Widget.size", 529)]
    pub fn size_<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        value: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        if value.lt(0)? {
            return Ok(());
        }
        slf.set_attr("_size", value)?;
        Ok(())
    }

    // unit_functions.py:535
    #[rt::frame(m, "Widget.nothing", 535)]
    pub fn nothing<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<()> {
        rt::builtins::print(m.py(), ("nothing of", slf.attr("name")?))?;
        Ok(())
    }

    // unit_functions.py:539
    #[rt::frame(m, "Widget.__setitem__", 539)]
    pub fn __setitem__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        key: &Bound<'py, PyAny>,
        value: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        rt::builtins::print(m.py(), ("setitem", key, value))?;
        Ok(())
    }

    // unit_functions.py:542
    #[rt::frame(m, "Widget.__call__", 542)]
    pub fn __call__<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<()> {
        rt::builtins::print(m.py(), ("called", slf.attr("name")?))?;
        Ok(())
    }

    // unit_functions.py:545
    #[rt::frame(m, "Widget.__del__", 545)]
    pub fn __del__<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<()> {
        rt::builtins::print(m.py(), ("del", slf.attr("name")?))?;
        Ok(())
    }
}

pub mod bad_init {
    use super::*;

    // unit_functions.py:560
    /// The body of `class BadInit`, run in its namespace `ns`.
    #[rt::frame(ns, "BadInit", 560)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __init__(slf as "self", flag))?;
        Ok(())
    }

    // unit_functions.py:561
    #[rt::frame(m, "BadInit.__init__", 561)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        flag: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        if flag.is_truthy()? {
            return Ok(());
        }
        Ok(())
    }
}

/// Executes the body of `unit_functions.py` in the module `m`.
#[rt::frame(m, "<module>", 1, frame)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "Functions that never return a value: their Rust fns return `PyResult<()>`,\n\
            and Python still sees `None` wherever the value is used.",
    )?;
    m.set_global("functools", rt::import(m, "functools")?)?;
    m.set_global("log", PyList::empty(py))?;
    rt::def!(m, note(label))?;
    rt::def!(m, proc(x))?;
    rt::def!(m, early(x))?;
    rt::def!(m, explicit_none())?;
    rt::def!(m, empty())?;
    rt::def!(m, only_docstring(), doc = "Does nothing.")?;
    rt::def!(m, mixed(x))?;
    rt::def!(m, tail_call(x))?;
    rt::def!(m, always_raises(message))?;
    rt::def!(m, loop_return(items, stop))?;
    rt::def!(m, while_return(n))?;
    rt::def!(m, match_return(value))?;
    rt::def!(
        m,
        with_defaults(
            a,
            b = 2,
            *rest,
            c = "c",
            items = PyList::empty(py),
            **options
        )
    )?;
    rt::def!(m, countdown(n))?;
    rt::def!(m, value(label))?;
    rt::builtins::print(py, ("--- procedures",))?;
    proc(m, &rt::int(py, 1))?;
    early(m, &rt::int(py, -1))?;
    early(m, &rt::int(py, 0))?;
    early(m, &rt::int(py, 1))?;
    empty(m)?;
    only_docstring(m)?;
    proc(m, &rt::int(py, 2))?;
    early(m, &rt::int(py, 0))?;
    explicit_none(m)?;
    empty(m)?;
    only_docstring(m)?;
    rt::builtins::print(py, (py.None(), py.None(), py.None(), py.None(), py.None()))?;
    rt::builtins::print(
        py,
        (
            mixed(m, &rt::int(py, 3))?,
            mixed(m, &rt::int(py, 0))?,
            tail_call(m, &rt::int(py, 4))?,
        ),
    )?;
    loop_return(m, PyList::new(py, [1, 2, 3])?.as_any(), &rt::int(py, 2))?;
    loop_return(m, PyList::new(py, [1])?.as_any(), &rt::int(py, 2))?;
    while_return(m, &rt::int(py, 2))?;
    match_return(m, &rt::int(py, 0))?;
    match_return(m, PyList::new(py, [1, 2])?.as_any())?;
    match_return(m, &rt::str(py, "x"))?;
    countdown(m, &rt::int(py, 3))?;
    rt::builtins::print(py, ("--- uses of the value",))?;
    proc(m, &rt::str(py, "assign"))?;
    m.set_global("x", py.None())?;
    rt::builtins::print(py, (m.global("x")?,))?;
    {
        proc(m, &rt::str(py, "chained"))?;
        let value2 = rt::none(py);
        m.set_global("a", &value2)?;
        m.set_global("b", &value2)?;
    }
    rt::builtins::print(py, (m.global("a")?, m.global("b")?))?;
    m.set_global("total", 1)?;
    {
        let value2 = m.global("total")?;
        proc(m, &rt::str(py, "augmented"))?;
        m.set_global(
            "total",
            rt::iadd(
                &value2,
                if rt::none(py).is_none() {
                    rt::int(py, 0)
                } else {
                    rt::int(py, 1)
                },
            )?,
        )?;
    }
    rt::builtins::print(py, (m.global("total")?,))?;
    proc(m, &rt::str(py, "argument"))?;
    rt::builtins::print(py, (py.None(),))?;
    let value2 = value(m, &rt::str(py, "before"))?;
    proc(m, &rt::str(py, "middle"))?;
    rt::builtins::print(py, (value2, py.None(), value(m, &rt::str(py, "after"))?))?;
    {
        let value2 = value(m, &rt::int(py, 1))?;
        proc(m, &rt::str(py, "tuple"))?;
        rt::builtins::print(
            py,
            (PyTuple::new(
                py,
                [&value2, &rt::none(py), &value(m, &rt::int(py, 2))?],
            )?,),
        )?;
    }
    {
        let value2 = value(m, &rt::int(py, 1))?;
        proc(m, &rt::str(py, "list"))?;
        rt::builtins::print(
            py,
            (PyList::new(
                py,
                [&value2, &rt::none(py), &value(m, &rt::int(py, 2))?],
            )?,),
        )?;
    }
    {
        let value2 = value(m, &rt::str(py, "k"))?;
        proc(m, &rt::str(py, "dict value"))?;
        proc(m, &rt::str(py, "dict key"))?;
        rt::builtins::print(
            py,
            ([
                (&value2, &rt::none(py)),
                (&rt::none(py), &value(m, &rt::str(py, "v"))?),
            ]
            .into_py_dict(py)?,),
        )?;
    }
    proc(m, &rt::str(py, "set"))?;
    rt::builtins::print(py, (PySet::new(py, [&rt::none(py)])?,))?;
    rt::builtins::print(
        py,
        (rt::list!(
            py,
            *PyList::new(py, [&value(m, &rt::int(py, 0))?])?,
            {
                proc(m, &rt::str(py, "starred display"))?;
                rt::none(py)
            }
        )?,),
    )?;
    proc(m, &rt::str(py, "or"))?;
    rt::builtins::print(
        py,
        (if false {
            rt::none(py)
        } else {
            rt::str(py, "fallback")
        },),
    )?;
    proc(m, &rt::str(py, "and"))?;
    rt::builtins::print(
        py,
        (if false {
            rt::str(py, "unused")
        } else {
            rt::none(py)
        },),
    )?;
    {
        let value2 = value(m, &rt::int(py, 0))?;
        rt::builtins::print(
            py,
            (if value2.is_truthy()? {
                value2
            } else {
                proc(m, &rt::str(py, "or rhs"))?;
                rt::none(py)
            },),
        )?;
    }
    {
        let value2 = value(m, &rt::int(py, 1))?;
        rt::builtins::print(
            py,
            (if value2.is_truthy()? {
                proc(m, &rt::str(py, "and rhs"))?;
                rt::none(py)
            } else {
                value2
            },),
        )?;
    }
    proc(m, &rt::str(py, "is"))?;
    proc(m, &rt::str(py, "is not"))?;
    rt::builtins::print(py, (rt::none(py).is_none(), !rt::none(py).is_none()))?;
    // noqa: E711
    proc(m, &rt::str(py, "eq"))?;
    let value2 = rt::none(py).rich_compare(py.None(), CompareOp::Eq)?;
    proc(m, &rt::str(py, "ne"))?;
    rt::builtins::print(
        py,
        (value2, rt::none(py).rich_compare(py.None(), CompareOp::Ne)?),
    )?;
    {
        let value2 =
            value(m, &rt::int(py, 1))?.rich_compare(value(m, &rt::int(py, 2))?, CompareOp::Lt)?;
        rt::builtins::print(
            py,
            (if value2.is_truthy()? {
                proc(m, &rt::str(py, "chain"))?;
                rt::bool(py, rt::none(py).is_none())
            } else {
                value2
            },),
        )?;
    }
    proc(m, &rt::str(py, "if"))?;
    if false {
        rt::builtins::print(py, ("truthy",))?;
    } else {
        rt::builtins::print(py, ("falsy",))?;
    }
    proc(m, &rt::str(py, "not"))?;
    if true {
        rt::builtins::print(py, ("not",))?;
    }
    m.set_global("count", 0)?;
    loop {
        proc(m, &rt::str(py, "while"))?;
        if !(true && m.global("count")?.lt(2)?) {
            break;
        }
        m.set_global("count", rt::iadd(&m.global("count")?, 1)?)?;
        py.check_signals()?;
    }
    loop {
        proc(m, &rt::str(py, "while false"))?;
        if true {
            break;
        }
        rt::builtins::print(py, ("never",))?;
        py.check_signals()?;
    }
    proc(m, &rt::str(py, "fstring"))?;
    rt::builtins::print(py, (rt::format(&rt::none(py), "")?,))?;
    proc(m, &rt::str(py, "spec"))?;
    let value2 = rt::format(rt::none(py).str()?.as_any(), ">6")?;
    proc(m, &rt::str(py, "repr"))?;
    let value3 = rt::none(py).repr()?;
    let value4 = rt::format(&value(m, &rt::str(py, "x"))?, "")?;
    proc(m, &rt::str(py, "y"))?;
    rt::builtins::print(
        py,
        (rt::fstring(
            py,
            (
                "[",
                value2,
                "] [",
                value3,
                "] ",
                value4,
                rt::format(&rt::none(py), "")?,
                rt::format(&value(m, &rt::str(py, "z"))?, "")?,
            ),
        )?,),
    )?;
    proc(m, &rt::str(py, "ifexp test"))?;
    rt::builtins::print(
        py,
        (if false {
            rt::str(py, "yes")
        } else {
            rt::str(py, "no")
        },),
    )?;
    rt::builtins::print(
        py,
        (if value(m, &rt::int(py, 1))?.is_truthy()? {
            proc(m, &rt::str(py, "ifexp body"))?;
            rt::none(py)
        } else {
            proc(m, &rt::str(py, "unused"))?;
            rt::none(py)
        },),
    )?;
    rt::builtins::print(
        py,
        (if value(m, &rt::int(py, 0))?.is_truthy()? {
            proc(m, &rt::str(py, "unused"))?;
            rt::none(py)
        } else {
            proc(m, &rt::str(py, "ifexp orelse"))?;
            rt::none(py)
        },),
    )?;
    proc(m, &rt::str(py, "assert"))?;
    if !rt::none(py).is_none() {
        return Err(frame.raise(PyAssertionError::new_err(())));
    }
    proc(m, &rt::str(py, "assert test"))?;
    if false {
        proc(m, &rt::str(py, "unused message"))?;
        return Err(frame.raise(PyAssertionError::new_err((rt::none(py).unbind(),))));
    }
    let list = PyList::empty(py);
    for i in rt::iter(&rt::builtins::range(py, (2,))?)? {
        let i = i?;
        proc(m, &i)?;
        list.append(py.None())?;
    }
    rt::builtins::print(py, (list,))?;
    let dict = PyDict::new(py);
    for i in rt::iter(&rt::builtins::range(py, (2,))?)? {
        let i = i?;
        proc(m, &i)?;
        dict.set_item(&i, py.None())?;
    }
    rt::builtins::print(py, (dict,))?;
    let list = PyList::empty(py);
    for i in rt::iter(&rt::builtins::range(py, (3,))?)? {
        let i = i?;
        proc(m, &i)?;
        if true {
            list.append(&i)?;
        }
    }
    rt::builtins::print(py, (list,))?;
    // unit_functions.py:158
    #[rt::frame(m, "<genexpr>", 158)]
    fn genexpr<'py>(
        m: &rt::Module<'py>,
        iter: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(i) = rt::next(iter)? {
            proc(m, &i)?;
            return Ok(Some(rt::none(m.py())));
        }
        Ok(None)
    }
    rt::builtins::print(
        py,
        (rt::builtins::list(
            py,
            (rt::genexpr!(
                m,
                genexpr(rt::builtins::range(py, (2,))?.try_iter()?)
            )?,),
        )?,),
    )?;
    proc(m, &rt::str(py, "str"))?;
    let value2 = rt::none(py).str()?;
    proc(m, &rt::str(py, "repr"))?;
    let value3 = rt::none(py).repr()?;
    proc(m, &rt::str(py, "bool"))?;
    let value4 = false;
    proc(m, &rt::str(py, "type"))?;
    rt::builtins::print(py, (value2, value3, value4, rt::none(py).get_type()))?;
    {
        let value2 = value(m, PyList::new(py, [1, 2])?.as_any())?;
        proc(m, &rt::str(py, "subscript"))?;
        rt::builtins::print(py, (value2.get_item(rt::none(py).is_none())?,))?;
    }
    proc(m, &rt::str(py, "len"))?;
    rt::builtins::print(py, (PyList::new(py, [&rt::none(py)])?.len(),))?;
    proc(m, &rt::str(py, "attribute"))?;
    rt::builtins::print(py, (rt::none(py).attr("__class__")?.attr("__name__")?,))?;
    match (|| -> PyResult<Bound<'_, PyAny>> {
        proc(m, &rt::str(py, "call the value"))?;
        rt::none(py).call0()
    })() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let exception = frame.catch(err);
            m.set_global("e", &exception)?;
            rt::builtins::print(py, ("TypeError", m.global("e")?))?;
            m.unbind_global("e")?;
        }
        Err(err) => return Err(err),
    }
    match (|| -> PyResult<Bound<'_, PyAny>> {
        proc(m, &rt::str(py, "add"))?;
        rt::none(py).add(value(m, &rt::int(py, 1))?)
    })() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let exception = frame.catch(err);
            m.set_global("e", &exception)?;
            rt::builtins::print(py, ("TypeError", m.global("e")?))?;
            m.unbind_global("e")?;
        }
        Err(err) => return Err(err),
    }
    match (|| -> PyResult<Bound<'_, PyAny>> {
        let value2 = value(m, &rt::int(py, 1))?;
        proc(m, &rt::str(py, "radd"))?;
        value2.add(py.None())
    })() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let exception = frame.catch(err);
            m.set_global("e", &exception)?;
            rt::builtins::print(py, ("TypeError", m.global("e")?))?;
            m.unbind_global("e")?;
        }
        Err(err) => return Err(err),
    }
    let value3 = value(m, &rt::str(py, "lhs"))?;
    proc(m, &rt::str(py, "in"))?;
    let value4 = PyList::new(py, [&rt::none(py)])?.contains(value3)?;
    proc(m, &rt::str(py, "not in"))?;
    rt::builtins::print(
        py,
        (
            value4,
            !value(m, PyList::new(py, [&rt::none(py)])?.as_any())?.contains(py.None())?,
        ),
    )?;
    proc(m, &rt::str(py, "walrus"))?;
    m.set_global("y", py.None())?;
    rt::builtins::print(py, (m.global("y")?,))?;
    proc(m, &rt::str(py, "unpack 1"))?;
    proc(m, &rt::str(py, "unpack 2"))?;
    let [p, q] = [rt::none(py), rt::none(py)];
    m.set_global("p", p)?;
    m.set_global("q", q)?;
    rt::builtins::print(py, (m.global("p")?, m.global("q")?))?;
    m.set_global("d", PyDict::new(py))?;
    proc(m, &rt::str(py, "subscript store"))?;
    m.global("d")?
        .set_item(value(m, &rt::str(py, "key"))?, py.None())?;
    {
        let value3 = value(m, &rt::str(py, "stored"))?;
        let d = m.global("d")?;
        proc(m, &rt::str(py, "key call"))?;
        d.set_item(py.None(), value3)?;
    }
    rt::builtins::print(py, (m.global("d")?,))?;
    {
        let d = m.global("d")?;
        proc(m, &rt::str(py, "del key"))?;
        d.del_item(py.None())?;
    }
    rt::builtins::print(py, (m.global("d")?,))?;
    proc(m, &rt::str(py, "for iterable"))?;
    for item in rt::iter(PyList::new(py, [&rt::none(py)])?.as_any())? {
        m.set_global("item", item?)?;
        rt::builtins::print(py, (m.global("item")?,))?;
    }
    // unit_functions.py:188
    #[rt::frame(m, "<lambda>", 188)]
    fn lambda<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        proc(m, &rt::str(py, "lambda"))?;
        Ok(rt::none(py))
    }
    let lambda_result = rt::function!(m, lambda(), qualname = "<lambda>")?.call0()?;
    m.set_global("lambda_result", &lambda_result)?;
    rt::builtins::print(py, (lambda_result,))?;
    // unit_functions.py:189
    fn lambda2<'py>(m: &rt::Module<'py>, v: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        Ok(v.clone())
    }
    proc(m, &rt::str(py, "lambda default"))?;
    rt::builtins::print(
        py,
        (rt::function!(m, lambda2(v = py.None()), qualname = "<lambda>")?.call0()?,),
    )?;
    proc(m, &rt::str(py, "nested"))?;
    proc(m, &rt::none(py))?;
    rt::builtins::print(py, (py.None(),))?;
    proc(m, &value(m, &rt::str(py, "inner"))?)?;
    early(m, &rt::int(py, 1))?;
    proc(m, &rt::none(py))?;
    rt::builtins::print(py, (py.None(), py.None()))?;
    proc(m, &rt::str(py, "statement argument"))?;
    note(m, &rt::none(py))?;
    rt::builtins::print(py, ("--- calls",))?;
    with_defaults(
        m,
        &rt::int(py, 1),
        &m.default("with_defaults", "b")?,
        &PyTuple::empty(py),
        &m.default("with_defaults", "c")?,
        &m.default("with_defaults", "items")?,
        &PyDict::new(py),
    )?;
    with_defaults(
        m,
        &rt::int(py, 1),
        &value(m, &rt::str(py, "b"))?,
        &PyTuple::empty(py),
        &rt::str(py, "C"),
        &m.default("with_defaults", "items")?,
        &PyDict::new(py),
    )?;
    {
        let a = value(m, &rt::int(py, 1))?;
        let b = value(m, &rt::int(py, 2))?;
        let rest = value(m, &rt::int(py, 3))?;
        let rest2 = value(m, &rt::int(py, 4))?;
        let c = value(m, &rt::str(py, "c"))?;
        let options = value(m, &rt::str(py, "z"))?;
        with_defaults(
            m,
            &a,
            &b,
            &PyTuple::new(py, [&rest, &rest2])?,
            &c,
            &m.default("with_defaults", "items")?,
            &[("z", &options)].into_py_dict(py)?,
        )?;
    }
    proc(m, &rt::str(py, "keyword argument"))?;
    with_defaults(
        m,
        &rt::none(py),
        &m.default("with_defaults", "b")?,
        &PyTuple::empty(py),
        &m.default("with_defaults", "c")?,
        &m.default("with_defaults", "items")?,
        &PyDict::new(py),
    )?;
    rt::builtins::print(py, (py.None(),))?;
    m.set_global("args", PyTuple::new(py, [1, 2, 3])?)?;
    m.set_global("options", [("c", "star"), ("k", "v")].into_py_dict(py)?)?;
    rt::call!(
        m.global("with_defaults")?,
        *m.global("args")?,
        **m.global("options")?
    )?;
    rt::builtins::print(
        py,
        (rt::call!(m.global("with_defaults")?, *m.global("args")?)?,),
    )?;
    let value3 = PyTuple::new(py, [20])?;
    m.global("with_defaults")?
        .set_attr("__defaults__", value3)?;
    with_defaults(
        m,
        &rt::int(py, 10),
        &m.default("with_defaults", "b")?,
        &PyTuple::empty(py),
        &m.default("with_defaults", "c")?,
        &m.default("with_defaults", "items")?,
        &PyDict::new(py),
    )?;
    with_defaults(
        m,
        &rt::int(py, 1),
        &m.default("with_defaults", "b")?,
        &PyTuple::empty(py),
        &m.default("with_defaults", "c")?,
        &m.default("with_defaults", "items")?,
        &PyDict::new(py),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("with_defaults")?.attr("__defaults__")?,
            m.global("with_defaults")?.attr("__kwdefaults__")?,
        ),
    )?;
    let __name__ = m.global("proc")?.attr("__name__")?;
    proc(m, &rt::int(py, 0))?;
    rt::builtins::print(
        py,
        (
            __name__,
            rt::none(py).is_none(),
            m.global("proc")?.method1("__call__", (1,))?,
        ),
    )?;
    match (|| m.global("proc")?.call0())() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let exception = frame.catch(err);
            m.set_global("e", &exception)?;
            rt::builtins::print(py, ("TypeError", m.global("e")?))?;
            m.unbind_global("e")?;
        }
        Err(err) => return Err(err),
    }
    match (|| -> PyResult<Bound<'_, PyAny>> {
        always_raises(m, &rt::str(py, "caught"))?;
        rt::builtins::print(py, (py.None(),))
    })() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyValueError>(py) => {
            let exception = frame.catch(err);
            m.set_global("e", &exception)?;
            rt::builtins::print(py, ("ValueError", m.global("e")?))?;
            m.unbind_global("e")?;
        }
        Err(err) => return Err(err),
    }
    match (|| -> PyResult<Bound<'_, PyAny>> {
        always_raises(m, &rt::str(py, "walrus in try"))?;
        m.set_global("stored", py.None())?;
        Ok(rt::none(py))
    })() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyValueError>(py) => {
            let exception = frame.catch(err);
            m.set_global("e", &exception)?;
            rt::builtins::print(py, ("ValueError", m.global("e")?))?;
            m.unbind_global("e")?;
        }
        Err(err) => return Err(err),
    }
    match (|| -> PyResult<Bound<'_, PyAny>> {
        proc(m, &rt::str(py, "walrus in try"))?;
        m.set_global("stored", py.None())?;
        Ok(rt::none(py))
    })() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyValueError>(py) => {
            let exception = frame.catch(err);
            m.set_global("e", &exception)?;
            rt::builtins::print(py, ("unreachable",))?;
            m.unbind_global("e")?;
        }
        Err(err) => return Err(err),
    }
    rt::builtins::print(py, (m.global("stored")?,))?;
    rt::builtins::print(py, ("--- inside functions",))?;
    rt::def!(m, uses())?;
    rt::builtins::print(py, (uses(m)?,))?;
    rt::def!(m, returns_unit_call(flag))?;
    rt::builtins::print(
        py,
        (
            returns_unit_call(m, &rt::bool(py, true))?,
            returns_unit_call(m, &rt::bool(py, false))?,
        ),
    )?;
    rt::def!(m, unit_calling_units(x))?;
    unit_calling_units(m, &rt::int(py, 7))?;
    rt::builtins::print(py, (py.None(),))?;
    rt::def!(m, order(x))?;
    rt::builtins::print(py, (order(m, &rt::str(py, "order"))?,))?;
    rt::builtins::print(py, ("--- try, finally and with",))?;
    m.set_global(
        "Manager",
        rt::class!(
            m,
            "Manager"(),
            manager::body,
            line = 272,
            attributes = ["name", "suppress"]
        )?,
    )?;
    rt::def!(m, try_finally(x))?;
    rt::def!(m, try_except(x))?;
    rt::def!(m, try_always_returns(x))?;
    rt::def!(m, try_except_returns(x))?;
    rt::def!(m, try_return_none(x))?;
    rt::def!(m, try_call(x))?;
    rt::def!(m, try_call_value(x))?;
    rt::def!(m, with_return(x))?;
    rt::def!(m, with_always_returns())?;
    rt::def!(m, with_suppressed())?;
    rt::def!(m, with_loop(items))?;
    rt::def!(m, nested_regions(x))?;
    rt::def!(m, finally_returns())?;
    rt::def!(m, loop_try(items))?;
    for item2 in rt::iter(&rt::builtins::range(py, (3,))?)? {
        m.set_global("n", item2?)?;
        try_finally(m, &m.global("n")?)?;
        rt::builtins::print(py, (py.None(),))?;
        try_except(m, &m.global("n")?)?;
        rt::builtins::print(py, (py.None(),))?;
        nested_regions(m, &m.global("n")?)?;
        rt::builtins::print(py, (py.None(),))?;
    }
    try_always_returns(m, &rt::int(py, 1))?;
    rt::builtins::print(py, (py.None(),))?;
    try_except_returns(m, &rt::str(py, "1"))?;
    try_except_returns(m, &rt::str(py, "x"))?;
    try_return_none(m, &rt::int(py, 1))?;
    rt::builtins::print(py, (py.None(), py.None(), py.None()))?;
    rt::builtins::print(py, (try_call(m, &rt::str(py, "t"))?,))?;
    rt::builtins::print(py, (try_call_value(m, &rt::str(py, "message"))?,))?;
    with_return(m, &rt::int(py, 0))?;
    with_return(m, &rt::int(py, 1))?;
    rt::builtins::print(py, (py.None(), py.None()))?;
    with_always_returns(m)?;
    rt::builtins::print(py, (py.None(),))?;
    with_suppressed(m)?;
    rt::builtins::print(py, (py.None(),))?;
    with_loop(
        m,
        PyList::new(py, ["one", "skip", "two", "stop", "three"])?.as_any(),
    )?;
    rt::builtins::print(py, (py.None(),))?;
    with_loop(m, PyList::new(py, ["one", "return", "two"])?.as_any())?;
    rt::builtins::print(py, (py.None(),))?;
    finally_returns(m)?;
    rt::builtins::print(py, (py.None(),))?;
    loop_try(m, PyList::new(py, [0, 1, 2, 4])?.as_any())?;
    rt::builtins::print(py, (py.None(),))?;
    loop_try(m, PyList::new(py, [0, 3, 4])?.as_any())?;
    rt::builtins::print(py, (py.None(),))?;
    rt::builtins::print(py, ("--- nested functions and closures",))?;
    rt::def!(m, counter())?;
    let [bump, show] = rt::unpack(&counter(m)?)?;
    m.set_global("bump", bump)?;
    m.set_global("show", show)?;
    rt::builtins::print(
        py,
        (m.global("bump")?.call1((2,))?, m.global("show")?.call0()?),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("bump")?.attr("__name__")?,
            m.global("bump")?.attr("__qualname__")?,
            m.global("bump")?.attr("__defaults__")?,
        ),
    )?;
    rt::def!(m, recursive_nested(n))?;
    rt::builtins::print(py, (recursive_nested(m, &rt::int(py, 2))?,))?;
    // noqa: E731
    // unit_functions.py:497
    #[rt::frame(m, "<lambda>", 497)]
    fn lambda3<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        proc(m, x)?;
        Ok(rt::none(m.py()))
    }
    m.set_global("call", rt::function!(m, lambda3(x), qualname = "<lambda>")?)?;
    rt::builtins::print(py, (m.global("call")?.call1(("from lambda",))?,))?;
    // unit_functions.py:499
    #[rt::frame(m, "<lambda>", 499)]
    fn lambda4<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        proc(m, &rt::str(py, "in list"))?;
        early(m, &rt::int(py, 0))?;
        Ok(PyList::new(py, [&rt::none(py), &rt::none(py)])?.into_any())
    }
    rt::builtins::print(
        py,
        (rt::function!(m, lambda4(), qualname = "<lambda>")?.call0()?,),
    )?;
    rt::builtins::print(py, ("--- methods",))?;
    m.set_global(
        "Widget",
        rt::class!(
            m,
            "Widget"(),
            widget::body,
            line = 504,
            attributes = ["_size", "name"]
        )?,
    )?;
    m.set_global("w", m.global("Widget")?.call1(("w",))?)?;
    rt::builtins::print(
        py,
        (
            m.global("w")?.method1("rename", ("w",))?,
            m.global("w")?.method1("rename", ("v",))?,
            m.global("w")?.attr("name")?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("Widget")?.method1("describe", ("static",))?,
            m.global("w")?.method1("describe", ("instance",))?,
            m.global("Widget")?.method0("register")?,
            m.global("Widget")?.attr("instances")?,
        ),
    )?;
    m.global("w")?.set_attr("size", 3)?;
    m.global("w")?.set_attr("size", -1)?;
    rt::builtins::print(
        py,
        (
            m.global("w")?.attr("size")?,
            m.global("w")?.attr("nothing")?,
        ),
    )?;
    m.global("w")?.set_item("k", "v")?;
    rt::builtins::print(
        py,
        (
            m.global("w")?.call0()?,
            m.global("Widget")?
                .attr("__init__")?
                .call1((m.global("w")?, "again"))?,
            m.global("w")?.attr("name")?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("Widget")?.attr("rename")?.attr("__qualname__")?,
            m.global("Widget")?
                .attr("rename")?
                .get_type()
                .attr("__name__")?,
        ),
    )?;
    m.set_global(
        "BadInit",
        rt::class!(m, "BadInit"(), bad_init::body, line = 560)?,
    )?;
    rt::builtins::print(
        py,
        (
            m.global("BadInit")?
                .call1((true,))?
                .get_type()
                .attr("__name__")?,
            m.global("BadInit")?
                .call1((false,))?
                .get_type()
                .attr("__name__")?,
        ),
    )?;
    m.set_global("w", py.None())?;
    rt::builtins::print(py, ("after del",))?;
    rt::builtins::print(py, ("--- decorators and callbacks",))?;
    rt::def!(m, traced(r#fn))?;
    rt::def!(m, announce(r#fn))?;
    rt::def!(m, @traced decorated(x, y = 1))?;
    rt::def!(m, @announce announced(x))?;
    rt::def!(m, empty_key(item))?;
    rt::def!(m, @functools.cache cached(x))?;
    rt::builtins::print(
        py,
        (
            m.global("decorated")?.call1((1,))?,
            rt::call!(m.global("decorated")?, 2, y = 3)?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("announced")?.call1((1,))?,
            m.global("announced")?.call1((0,))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("cached")?.call1((1,))?,
            m.global("cached")?.call1((1,))?,
            m.global("cached")?.call1((2,))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (rt::builtins::list(
            py,
            (rt::builtins::map(
                py,
                (m.global("proc")?, PyList::new(py, [1, 2])?),
            )?,),
        )?,),
    )?;
    rt::builtins::print(
        py,
        (
            rt::call!(
                rt::builtin(py, "sorted")?,
                PyList::new(py, [3])?,
                key = m.global("early")?
            )?,
            rt::call!(
                rt::builtin(py, "min")?,
                PyList::new(py, [1])?,
                key = m.global("empty_key")?
            )?,
        ),
    )?;
    rt::builtins::print(
        py,
        (rt::builtins::list(
            py,
            (rt::builtins::filter(
                py,
                (m.global("proc")?, PyList::new(py, [1, 2])?),
            )?,),
        )?,),
    )?;
    // unit_functions.py:616
    #[rt::frame(m, "<lambda>", 616)]
    fn lambda5<'py>(
        m: &rt::Module<'py>,
        acc: &Bound<'py, PyAny>,
        item: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        proc(m, item)?;
        Ok(rt::none(m.py()))
    }
    rt::builtins::print(
        py,
        (m.global("functools")?.attr("reduce")?.call1((
            rt::function!(m, lambda5(acc, item), qualname = "<lambda>")?,
            PyList::new(py, [1, 2])?,
            0,
        ))?,),
    )?;
    rt::builtins::print(
        py,
        (rt::call!(
            m.global("functools")?.attr("partial")?,
            m.global("with_defaults")?,
            1,
            c = "partial"
        )?
        .call0()?,),
    )?;
    rt::builtins::print(py, (m.global("log")?,))?;
    rt::builtins::print(py, ("--- uncaught",))?;
    rt::def!(m, fails_late(x))?;
    rt::def!(m, outer())?;
    outer(m)?;
    Ok(())
}
