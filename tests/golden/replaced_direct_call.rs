use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// replaced_direct_call.py:9
pub fn f<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "f", 9, |frame| {
        frame.line(10);
        Ok(PyTuple::new(m.py(), [a, b])?.into_any())
    })
}

// replaced_direct_call.py:13
pub fn g<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    Ok(x.clone())
}

/// Executes the body of `replaced_direct_call.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        // A direct call whose target is replaced from another module (here
        // `unittest.mock`, patching by name). CPython calls the replacement;
        // transpiled code still calls `f` and can not take `f`'s defaults from the
        // replacement (design §12).
        // steelix-test: expect-error TypeError: f() was replaced by <function g at ADDRESS>, but transpiled code calls the original directly and can not take the default of its parameter 'b' from the replacement
        frame.line(6);
        let unittest = rt::import_names(m, "unittest", &["mock"])?;
        m.set_global("mock", rt::import_from(&unittest, "mock")?)?;
        rt::def!(m, f(a, b = PyList::empty(py)))?;
        rt::def!(m, g(x))?;
        frame.line(17);
        rt::builtins::print(py, (f(m, &rt::int(py, 1), &m.default("f", "b")?)?,))?;
        frame.line(18);
        m.global("mock")?
            .getattr("patch")?
            .call1(("__main__.f", m.global("g")?))?
            .call_method0("start")?;
        frame.line(19);
        rt::builtins::print(py, (f(m, &rt::int(py, 1), &m.default("f", "b")?)?,))?;
        Ok(())
    })
}
