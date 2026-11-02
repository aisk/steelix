use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;

// shadowed_builtin.py:4
pub fn len<'py>(py: Python<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::int(py, 42))
}

// shadowed_builtin.py:8
pub fn r#use<'py>(py: Python<'py>, xs: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    len(py, xs)?.add(1)
}

/// Executes the body of `shadowed_builtin.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    globals.set_item(
        "__doc__",
        "A builtin rebound at module level is no longer special-cased.",
    )?;
    rt::def!(globals, len(x))?;
    rt::def!(globals, r#use(xs))?;
    globals.set_item("str", rt::builtin(py, "repr")?)?;
    rt::builtins::print(
        py,
        (
            r#use(py, PyList::new(py, [1, 2])?.as_any())?,
            len(py, &rt::str(py, "abc"))?,
            rt::global(globals, "str")?.call1(("quoted",))?,
        ),
    )?;
    Ok(())
}
