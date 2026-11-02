use pyo3::prelude::*;
use steelix_runtime as rt;

// recursion.py:1
pub fn is_even<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let _guard = rt::RecursionGuard::enter(py)?;
    if n.eq(0)? {
        return Ok(rt::bool(py, true));
    }
    is_odd(m, &n.sub(1)?)
}

// recursion.py:7
pub fn is_odd<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let _guard = rt::RecursionGuard::enter(py)?;
    if n.eq(0)? {
        return Ok(rt::bool(py, false));
    }
    is_even(m, &n.sub(1)?)
}

// recursion.py:13
pub fn ackermann<'py>(
    module: &rt::Module<'py>,
    m: &Bound<'py, PyAny>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = module.py();
    let _guard = rt::RecursionGuard::enter(py)?;
    if m.eq(0)? {
        return n.add(1);
    }
    if n.eq(0)? {
        return ackermann(module, &m.sub(1)?, &rt::int(py, 1));
    }
    ackermann(module, &m.sub(1)?, &ackermann(module, m, &n.sub(1)?)?)
}

/// Executes the body of `recursion.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::def!(m, is_even(n))?;
    rt::def!(m, is_odd(n))?;
    rt::def!(m, ackermann(m, n))?;
    rt::builtins::print(
        py,
        (
            is_even(m, &rt::int(py, 10))?,
            is_odd(m, &rt::int(py, 7))?,
            is_even(m, &rt::int(py, 7))?,
        ),
    )?;
    rt::builtins::print(py, (ackermann(m, &rt::int(py, 2), &rt::int(py, 3))?,))?;
    Ok(())
}
