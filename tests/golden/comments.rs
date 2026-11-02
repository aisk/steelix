use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;

// A module level comment before a function.
// comments.py:7
pub fn total<'py>(m: &rt::Module<'py>, items: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    // Sum the items one by one.
    let mut result = rt::int(py, 0);
    for item in items.try_iter()? {
        let item = item?;
        py.check_signals()?;
        // Skip negative numbers.
        if item.lt(0)? {
            continue;
        }
        // trailing comments stay with their statement
        result = rt::iadd(&result, &item)?;
    }
    Ok(result)
}

/// Executes the body of `comments.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "Python comments are kept in the generated Rust code.",
    )?;
    rt::def!(m, total(items))?;
    // Comment before a call.
    rt::builtins::print(py, (total(m, PyList::new(py, [1, -2, 3])?.as_any())?,))?;
    // Comment at the end of the module.
    Ok(())
}
