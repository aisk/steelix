use pyo3::exceptions::PyStopIteration;
use pyo3::prelude::*;
use pyo3::types::PyIterator;
use steelix_runtime as rt;

// genexpr_stop_iteration.py:5
#[rt::frame(m, "boom", 5, frame)]
pub fn boom<'py>(m: &rt::Module<'py>, v: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    if v.eq(1)? {
        return Err(frame.raise(PyStopIteration::new_err("inner")));
    }
    Ok(v.clone())
}

// genexpr_stop_iteration.py:11
#[rt::frame(m, "run", 11, frame)]
pub fn run<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    // genexpr_stop_iteration.py:12
    #[rt::frame(m, "run.<locals>.<genexpr>", 12)]
    fn genexpr<'py>(
        m: &rt::Module<'py>,
        iter: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(v) = rt::next(iter)? {
            return Ok(Some(boom(m, &v)?));
        }
        Ok(None)
    }
    rt::builtins::list(
        py,
        (rt::genexpr!(
            frame,
            genexpr(rt::builtins::range(py, (3,))?.try_iter()?)
        )?,),
    )
}

/// Executes the body of `genexpr_stop_iteration.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    m.set_global(
        "__doc__",
        "A `StopIteration` raised in a generator expression becomes a\n\
            `RuntimeError` (PEP 479); the traceback chains both.",
    )?;
    rt::def!(m, boom(v))?;
    rt::def!(m, run())?;
    run(m)?;
    Ok(())
}
