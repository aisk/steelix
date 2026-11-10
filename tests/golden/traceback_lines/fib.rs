use pyo3::prelude::*;
use steelix_runtime as rt;

// fib.py:1
#[rt::frame(m, "fib", 1, frame, recursive)]
pub fn fib<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    frame.line(2);
    if n.lt(2)? {
        return Ok(n.clone());
    }
    frame.line(4);
    fib(m, &n.sub(1)?)?.add(fib(m, &n.sub(2)?)?)
}

/// Executes the body of `fib.py` in the module `m`.
#[rt::frame(m, "<module>", 1, frame)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::def!(m, fib(n))?;
    frame.line(7);
    rt::builtins::print(py, (fib(m, &rt::int(py, 10))?,))?;
    Ok(())
}
