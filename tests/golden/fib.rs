use pyo3::prelude::*;
use steelix_runtime as rt;

// fib.py:1
#[rt::frame(m, "fib", 1, recursive)]
pub fn fib<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    if n.lt(2)? {
        return Ok(n.clone());
    }
    fib(m, &n.sub(1)?)?.add(fib(m, &n.sub(2)?)?)
}

/// Executes the body of `fib.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::def!(m, fib(n))?;
    rt::builtins::print(py, (fib(m, &rt::int(py, 10))?,))?;
    Ok(())
}
