use pyo3::exceptions::PyStopIteration;
use pyo3::prelude::*;
use pyo3::types::PyIterator;
use steelix_runtime as rt;

// genexpr_stop_iteration.py:5
pub fn boom<'py>(m: &rt::Module<'py>, v: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "boom", 5, |frame| {
        if v.eq(1)? {
            return Err(frame.raise(PyStopIteration::new_err("inner")));
        }
        Ok(v.clone())
    })
}

// genexpr_stop_iteration.py:11
pub fn run<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "run", 11, |_| {
        // genexpr_stop_iteration.py:12
        fn genexpr<'py>(
            m: &rt::Module<'py>,
            iter: &Bound<'py, PyIterator>,
        ) -> PyResult<Option<Bound<'py, PyAny>>> {
            rt::frame!(m, "run.<locals>.<genexpr>", 12, |_| {
                if let Some(v) = rt::next(iter)? {
                    return Ok(Some(boom(m, &v)?));
                }
                Ok(None)
            })
        }
        rt::builtin(py, "list")?.call1((rt::genexpr!(
            m,
            genexpr(rt::builtin(py, "range")?.call1((3,))?.try_iter()?),
            qualname = "run.<locals>.<genexpr>"
        )?,))
    })
}

/// Executes the body of `genexpr_stop_iteration.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global("__doc__", "A `StopIteration` raised in a generator expression becomes a\n`RuntimeError` (PEP 479); the traceback chains both.")?;
        rt::def!(m, boom(v))?;
        rt::def!(m, run())?;
        run(m)?;
        Ok(())
    })
}
