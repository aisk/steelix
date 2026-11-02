use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;

// A module level comment before a function.
// comments.py:7
pub fn total<'py>(m: &rt::Module<'py>, items: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "total", 7, |frame| {
        // Sum the items one by one.
        let mut result = rt::int(py, 0);
        frame.line(10);
        for item in items.try_iter()? {
            let item = item?;
            // Skip negative numbers.
            frame.line(12);
            if item.lt(0)? {
                py.check_signals()?;
                frame.line(10);
                continue;
            }
            // trailing comments stay with their statement
            frame.line(14);
            result = rt::iadd(&result, &item)?;
            py.check_signals()?;
            frame.line(10);
        }
        Ok(result)
    })
}

/// Executes the body of `comments.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "Python comments are kept in the generated Rust code.",
        )?;
        rt::def!(m, total(items))?;
        // Comment before a call.
        frame.line(19);
        rt::builtins::print(py, (total(m, PyList::new(py, [1, -2, 3])?.as_any())?,))?;
        // Comment at the end of the module.
        Ok(())
    })
}
