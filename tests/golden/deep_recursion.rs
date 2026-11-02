use pyo3::prelude::*;
use steelix_runtime as rt;

// deep_recursion.py:6
pub fn depth<'py>(py: Python<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let _guard = rt::RecursionGuard::enter(py)?;
    depth(py, &n.add(1)?)
}

/// Executes the body of `deep_recursion.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    rt::def!(globals, depth(n))?;
    rt::builtins::print(py, ("start",))?;
    depth(py, &rt::int(py, 0))?;
    Ok(())
}
