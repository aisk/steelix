use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// replaced_direct_call.py:8
pub fn f<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(PyTuple::new(m.py(), [a, b])?.into_any())
}

// replaced_direct_call.py:12
pub fn g<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    Ok(x.clone())
}

/// Executes the body of `replaced_direct_call.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    // A direct call whose target is replaced through the module object.
    // CPython calls the replacement; transpiled code still calls `f` and can not
    // take `f`'s defaults from the replacement (design §12).
    // steelix-test: expect-error TypeError: f() was replaced by <function g at ADDRESS>, but transpiled code calls the original directly and can not take the default of its parameter 'b' from the replacement
    m.set_global("sys", rt::import(m, "sys")?)?;
    rt::def!(m, f(a, b = PyList::empty(py)))?;
    rt::def!(m, g(x))?;
    rt::builtins::print(py, (f(m, &rt::int(py, 1), &m.default("f", "b")?)?,))?;
    m.set_global(
        "me",
        m.global("sys")?
            .getattr("modules")?
            .get_item(m.global("__name__")?)?,
    )?;
    let g2 = m.global("g")?;
    m.global("me")?.setattr("f", &g2)?;
    rt::builtins::print(py, (f(m, &rt::int(py, 1), &m.default("f", "b")?)?,))?;
    Ok(())
}
