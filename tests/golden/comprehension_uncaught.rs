use pyo3::prelude::*;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use steelix_runtime as rt;

// comprehension_uncaught.py:6
pub fn totals<'py>(m: &rt::Module<'py>, rows: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "totals", 6, |_| {
        // comprehension_uncaught.py:7
        fn genexpr<'py>(
            m: &rt::Module<'py>,
            row: &Bound<'py, PyIterator>,
        ) -> PyResult<Option<Bound<'py, PyAny>>> {
            rt::frame!(m, "totals.<locals>.<genexpr>", 7, |_| {
                if let Some(c) = rt::next(row)? {
                    return Ok(Some(rt::int(m.py(), 10).floor_div(&c)?));
                }
                Ok(None)
            })
        }
        let list = PyList::empty(py);
        for row in rows.try_iter()? {
            let row = row?;
            list.append(rt::builtin(py, "sum")?.call1((rt::genexpr!(
                m,
                genexpr(row.try_iter()?),
                qualname = "totals.<locals>.<genexpr>"
            )?,))?)?;
            py.check_signals()?;
        }
        Ok(list.into_any())
    })
}

/// Executes the body of `comprehension_uncaught.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global(
            "__doc__",
            "An uncaught exception raised inside a generator expression that a list\n\
                comprehension consumes: the traceback has the comprehension's line in the\n\
                function and the `<genexpr>` frame.",
        )?;
        rt::def!(m, totals(rows))?;
        rt::builtins::print(
            py,
            (totals(
                m,
                PyList::new(py, [PyList::new(py, [1, 2])?.as_any()])?.as_any(),
            )?,),
        )?;
        rt::builtins::print(
            py,
            (totals(
                m,
                PyList::new(
                    py,
                    [
                        PyList::new(py, [1])?.as_any(),
                        PyList::new(py, [0])?.as_any(),
                    ],
                )?
                .as_any(),
            )?,),
        )?;
        Ok(())
    })
}
