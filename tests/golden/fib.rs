use pyo3::prelude::*;
use steelix_runtime as rt;

// fib.py:1
pub fn fib<'py>(py: Python<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let _guard = rt::RecursionGuard::enter(py)?;
    if n.lt(2)? {
        return Ok(n.clone());
    }
    fib(py, &n.sub(1)?)?.add(fib(py, &n.sub(2)?)?)
}

/// Executes the body of `fib.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    rt::def!(globals, fib(n))?;
    rt::builtins::print(py, (fib(py, &rt::int(py, 10))?,))?;
    Ok(())
}
