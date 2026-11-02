use pyo3::prelude::*;
use steelix_runtime as rt;

// steelix-test: compare-exception-type-only
// CPython reports "maximum recursion depth exceeded"; transpiled code is only
// bounded by the C stack and reports "Stack overflow (used N kB)".
// deep_recursion.py:6
pub fn depth<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let _guard = rt::RecursionGuard::enter(m.py())?;
    depth(m, &n.add(1)?)
}

/// Executes the body of `deep_recursion.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::def!(m, depth(n))?;
    rt::builtins::print(py, ("start",))?;
    depth(m, &rt::int(py, 0))?;
    Ok(())
}
