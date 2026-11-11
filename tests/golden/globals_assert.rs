use pyo3::exceptions::PyAssertionError;
use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// globals_assert.py:9
#[rt::frame(m, "bump", 9)]
pub fn bump<'py>(m: &rt::Module<'py>, step: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    m.set_global("counter", m.global("counter")?.add(step)?)?;
    m.set_global("counter", rt::iadd(&m.global("counter")?, 1)?)?;
    m.global("counter")
}

// globals_assert.py:16
#[rt::frame(m, "remember", 16)]
pub fn remember<'py>(m: &rt::Module<'py>, name: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    m.global("names")?.method1("append", (name,))?;
    Ok(rt::int(m.py(), m.global("names")?.len()?))
}

// globals_assert.py:21
#[rt::frame(m, "reset", 21)]
pub fn reset<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    m.set_global("counter", 0)?;
    m.set_global("names", PyList::empty(py))?;
    m.del_global("counter")?;
    Ok(rt::none(py))
}

// globals_assert.py:28
#[rt::frame(m, "check", 28, frame)]
pub fn check<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    if !x.gt(0)? {
        return Err(frame.raise(PyAssertionError::new_err(())));
    }
    if !x.lt(100)? {
        return Err(frame.raise(PyAssertionError::new_err("too big")));
    }
    if !x.is_instance(&rt::builtin(py, "int")?)? {
        return Err(frame.raise(PyAssertionError::new_err((rt::fstring(
            py,
            ("not an int: ", x.repr()?),
        )?
        .unbind(),))));
    }
    Ok(x.clone())
}

// globals_assert.py:36
#[rt::frame(m, "define", 36)]
pub fn define<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    m.set_global("late", "defined late")?;
    Ok(rt::none(m.py()))
}

/// Executes the body of `globals_assert.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "The global statement, module globals used from functions, pass and assert.",
    )?;
    m.set_global("sys", rt::import(m, "sys")?)?;
    m.set_global("counter", 0)?;
    m.set_global("names", PyList::empty(py))?;
    rt::def!(m, bump(step))?;
    rt::def!(m, remember(name))?;
    rt::def!(m, reset())?;
    rt::def!(m, check(x))?;
    rt::def!(m, define())?;
    rt::builtins::print(
        py,
        (
            bump(m, &rt::int(py, 2))?,
            bump(m, &rt::int(py, 3))?,
            m.global("counter")?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            remember(m, &rt::str(py, "a"))?,
            remember(m, &rt::str(py, "b"))?,
            m.global("names")?,
        ),
    )?;
    reset(m)?;
    rt::builtins::print(
        py,
        (
            rt::builtins::hasattr(
                py,
                (
                    m.global("sys")?
                        .attr("modules")?
                        .get_item(m.global("__name__")?)?,
                    "counter",
                ),
            )?,
            m.global("names")?,
        ),
    )?;
    define(m)?;
    rt::builtins::print(py, (m.global("late")?,))?;
    rt::builtins::print(py, (check(m, &rt::int(py, 5))?,))?;
    check(m, &rt::int(py, 500))?;
    Ok(())
}
