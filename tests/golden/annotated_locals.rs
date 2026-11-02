use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;

// annotated_locals.py:4
pub fn describe<'py>(
    m: &rt::Module<'py>,
    items: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "describe", 4, |frame| {
        let mut label: Option<Bound<'py, PyAny>> = None;
        frame.line(5);
        let count = rt::int(py, items.len()?);
        let mut total = rt::int(py, 0);
        frame.line(8);
        for item in items.try_iter()? {
            let item = item?;
            frame.line(9);
            total = rt::iadd(&total, &item)?;
            py.check_signals()?;
            frame.line(8);
        }
        frame.line(10);
        if count.is_truthy()? {
            label = Some(rt::str(py, "average"));
            frame.line(12);
            return rt::local(&label, "label")?
                .add(" ")?
                .add(total.div(&count)?.str()?);
        }
        frame.line(13);
        Ok(rt::local(&label, "label")?.clone())
    })
}

/// Executes the body of `annotated_locals.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "Annotated assignments in functions: the annotations are never evaluated.",
        )?;
        rt::def!(m, describe(items: "list") -> "str")?;
        frame.line(16);
        rt::builtins::print(py, (describe(m, PyList::new(py, [1, 2, 3])?.as_any())?,))?;
        frame.line(17);
        rt::builtins::print(py, (describe(m, PyList::empty(py).as_any())?,))?;
        Ok(())
    })
}
