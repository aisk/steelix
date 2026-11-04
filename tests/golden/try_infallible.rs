use pyo3::exceptions::PyException;
use pyo3::exceptions::PyNameError;
use pyo3::exceptions::PyUnboundLocalError;
use pyo3::prelude::*;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// `try` bodies that can not raise: the handlers are dead code.
// try_infallible.py:4
pub fn constant<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let x = rt::int(m.py(), 1);
    Ok(x)
}

// try_infallible.py:12
pub fn local_copy<'py>(m: &rt::Module<'py>, a: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "local_copy", 12, |_| {
        let b = a.clone();
        let c = b.clone();
        rt::builtins::print(m.py(), ("else runs",))?;
        Ok(c)
    })
}

// try_infallible.py:24
pub fn nothing<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "nothing", 24, |_| {
        let mut i: Option<Bound<'py, PyAny>> = None;
        for item in rt::builtin(py, "range")?.call1((3,))?.try_iter()? {
            i = Some(item?);
            py.check_signals()?;
            continue;
        }
        Ok(rt::local(&i, "i")?.clone())
    })
}

// try_infallible.py:35
pub fn pair<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "pair", 35, |frame| {
        let mut t: Option<Bound<'py, PyAny>> = None;
        let result = (|| -> PyResult<()> {
            t = Some(PyTuple::new(py, [a, b])?.into_any());
            Ok(())
        })();
        let finally = frame.finally(result);
        rt::builtins::print(py, ("finally",))?;
        finally.resume()?;
        Ok(rt::local(&t, "t")?.clone())
    })
}

// try_infallible.py:45
pub fn returned<'py>(m: &rt::Module<'py>, a: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    Ok(a.clone())
}

// try_infallible.py:52
pub fn maybe_unbound<'py>(
    m: &rt::Module<'py>,
    flag: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "maybe_unbound", 52, |_| {
        let mut y: Option<Bound<'py, PyAny>> = None;
        if flag.is_truthy()? {
            y = Some(rt::str(py, "set"));
        }
        let z = match (|| -> PyResult<Bound<'py, PyAny>> { Ok(rt::local(&y, "y")?.clone()) })() {
            Ok(value) => value,
            Err(err) if err.is_instance_of::<PyUnboundLocalError>(py) => rt::str(py, "unbound"),
            Err(err) => return Err(err),
        };
        Ok(z)
    })
}

// try_infallible.py:62
pub fn deleted_in_loop<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "deleted_in_loop", 62, |_| {
        let mut v = Some(rt::int(py, 0));
        let mut w: Bound<'py, PyAny>;
        for underscore in rt::builtin(py, "range")?.call1((n,))?.try_iter()? {
            let underscore = underscore?;
            w = rt::local(&v, "v")?.clone();
            v = None;
            v = Some(w.clone());
            py.check_signals()?;
        }
        Ok(rt::local(&v, "v")?.clone())
    })
}

// try_infallible.py:75
pub fn captured<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "captured", 75, |_| {
        let k = rt::int(m.py(), 5);
        // try_infallible.py:78
        fn inner<'py>(m: &rt::Module<'py>, k: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            let py = m.py();
            rt::frame!(m, "captured.<locals>.inner", 78, |_| {
                match Ok::<_, PyErr>(k.clone()) {
                    Ok(value) => Ok(value),
                    Err(err) if err.is_instance_of::<PyNameError>(py) => Ok(rt::none(py)),
                    Err(err) => Err(err),
                }
            })
        }
        let inner = rt::function!(m, inner[k](), qualname = "captured.<locals>.inner")?;
        inner.call0()
    })
}

/// Executes the body of `try_infallible.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        rt::def!(m, constant())?;
        rt::def!(m, local_copy(a))?;
        rt::def!(m, nothing())?;
        rt::def!(m, pair(a, b))?;
        rt::def!(m, returned(a))?;
        rt::def!(m, maybe_unbound(flag))?;
        rt::def!(m, deleted_in_loop(n))?;
        rt::def!(m, captured())?;
        rt::builtins::print(
            py,
            (
                constant(m)?,
                local_copy(m, &rt::int(py, 3))?,
                nothing(m)?,
                pair(m, &rt::int(py, 1), &rt::int(py, 2))?,
                returned(m, &rt::str(py, "r"))?,
            ),
        )?;
        rt::builtins::print(
            py,
            (
                maybe_unbound(m, &rt::bool(py, true))?,
                maybe_unbound(m, &rt::bool(py, false))?,
                deleted_in_loop(m, &rt::int(py, 2))?,
                captured(m)?,
            ),
        )?;
        match Ok::<_, PyErr>(rt::int(py, 10)) {
            Ok(value) => {
                m.set_global("g", &value)?;
            }
            Err(err) if err.is_instance_of::<PyException>(py) => {
                let _caught = frame.catch(err);
                m.set_global("g", 0)?;
            }
            Err(err) => return Err(err),
        }
        rt::builtins::print(py, (m.global("g")?,))?;
        Ok(())
    })
}
