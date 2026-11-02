use pyo3::exceptions::PyAssertionError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use steelix_runtime as rt;

// globals_assert.py:9
pub fn bump<'py>(
    py: Python<'py>,
    globals: &Bound<'py, PyDict>,
    step: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    globals.set_item("counter", rt::global(globals, "counter")?.add(step)?)?;
    globals.set_item("counter", rt::iadd(&rt::global(globals, "counter")?, 1)?)?;
    rt::global(globals, "counter")
}

// globals_assert.py:16
pub fn remember<'py>(
    py: Python<'py>,
    globals: &Bound<'py, PyDict>,
    name: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::global(globals, "names")?.call_method1("append", (name,))?;
    Ok(rt::int(py, rt::global(globals, "names")?.len()?))
}

// globals_assert.py:21
pub fn reset<'py>(py: Python<'py>, globals: &Bound<'py, PyDict>) -> PyResult<Bound<'py, PyAny>> {
    globals.set_item("counter", 0)?;
    globals.set_item("names", PyList::empty(py))?;
    rt::del_global(globals, "counter")?;
    Ok(rt::none(py))
}

// globals_assert.py:28
pub fn check<'py>(py: Python<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    if !x.gt(0)? {
        return Err(PyAssertionError::new_err(()));
    }
    if !x.lt(100)? {
        return Err(PyAssertionError::new_err("too big"));
    }
    if !x.is_instance(&rt::builtin(py, "int")?)? {
        return Err(PyAssertionError::new_err((rt::fstring(
            py,
            ("not an int: ", x.repr()?),
        )?
        .unbind(),)));
    }
    Ok(x.clone())
}

// globals_assert.py:36
pub fn define<'py>(py: Python<'py>, globals: &Bound<'py, PyDict>) -> PyResult<Bound<'py, PyAny>> {
    globals.set_item("late", "defined late")?;
    Ok(rt::none(py))
}

/// Executes the body of `globals_assert.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    globals.set_item(
        "__doc__",
        "The global statement, module globals used from functions, pass and assert.",
    )?;
    globals.set_item("sys", rt::import(globals, "sys")?)?;
    globals.set_item("counter", 0)?;
    globals.set_item("names", PyList::empty(py))?;
    rt::def!(globals, bump(globals, step))?;
    rt::def!(globals, remember(globals, name))?;
    rt::def!(globals, reset(globals))?;
    rt::def!(globals, check(x))?;
    rt::def!(globals, define(globals))?;
    rt::builtins::print(
        py,
        (
            bump(py, globals, &rt::int(py, 2))?,
            bump(py, globals, &rt::int(py, 3))?,
            rt::global(globals, "counter")?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            remember(py, globals, &rt::str(py, "a"))?,
            remember(py, globals, &rt::str(py, "b"))?,
            rt::global(globals, "names")?,
        ),
    )?;
    reset(py, globals)?;
    rt::builtins::print(
        py,
        (
            rt::builtin(py, "hasattr")?.call1((
                rt::global(globals, "sys")?
                    .getattr("modules")?
                    .get_item(rt::global(globals, "__name__")?)?,
                "counter",
            ))?,
            rt::global(globals, "names")?,
        ),
    )?;
    define(py, globals)?;
    rt::builtins::print(py, (rt::global(globals, "late")?,))?;
    rt::builtins::print(py, (check(py, &rt::int(py, 5))?,))?;
    check(py, &rt::int(py, 500))?;
    Ok(())
}
