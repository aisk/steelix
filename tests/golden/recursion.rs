use pyo3::prelude::*;
use steelix_runtime as rt;

// recursion.py:1
pub fn is_even<'py>(py: Python<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let _guard = rt::RecursionGuard::enter(py)?;
    if n.eq(0)? {
        return Ok(rt::bool(py, true));
    }
    is_odd(py, &n.sub(1)?)
}

// recursion.py:7
pub fn is_odd<'py>(py: Python<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let _guard = rt::RecursionGuard::enter(py)?;
    if n.eq(0)? {
        return Ok(rt::bool(py, false));
    }
    is_even(py, &n.sub(1)?)
}

// recursion.py:13
pub fn ackermann<'py>(
    py: Python<'py>,
    m: &Bound<'py, PyAny>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let _guard = rt::RecursionGuard::enter(py)?;
    if m.eq(0)? {
        return n.add(1);
    }
    if n.eq(0)? {
        return ackermann(py, &m.sub(1)?, &rt::int(py, 1));
    }
    ackermann(py, &m.sub(1)?, &ackermann(py, m, &n.sub(1)?)?)
}

/// Executes the body of `recursion.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    rt::def!(globals, is_even(n))?;
    rt::def!(globals, is_odd(n))?;
    rt::def!(globals, ackermann(m, n))?;
    rt::builtins::print(
        py,
        (
            is_even(py, &rt::int(py, 10))?,
            is_odd(py, &rt::int(py, 7))?,
            is_even(py, &rt::int(py, 7))?,
        ),
    )?;
    rt::builtins::print(py, (ackermann(py, &rt::int(py, 2), &rt::int(py, 3))?,))?;
    Ok(())
}
