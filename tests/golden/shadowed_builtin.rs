use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;

// shadowed_builtin.py:4
pub fn len<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    Ok(rt::int(m.py(), 42))
}

// shadowed_builtin.py:8
pub fn r#use<'py>(m: &rt::Module<'py>, xs: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "use", 8, |frame| {
        frame.line(9);
        len(m, xs)?.add(1)
    })
}

/// Executes the body of `shadowed_builtin.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "A builtin rebound at module level is no longer special-cased.",
        )?;
        rt::def!(m, len(x))?;
        rt::def!(m, r#use(xs))?;
        frame.line(12);
        m.set_global("str", rt::builtin(py, "repr")?)?;
        frame.line(13);
        rt::builtins::print(
            py,
            (
                r#use(m, PyList::new(py, [1, 2])?.as_any())?,
                len(m, &rt::str(py, "abc"))?,
                m.global("str")?.call1(("quoted",))?,
            ),
        )?;
        Ok(())
    })
}
