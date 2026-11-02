use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;

// annotated_locals.py:4
pub fn describe<'py>(py: Python<'py>, items: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let mut label: Option<Bound<'py, PyAny>> = None;
    let count = rt::int(py, items.len()?);
    let mut total = rt::int(py, 0);
    for item in items.try_iter()? {
        let item = item?;
        total = rt::iadd(&total, &item)?;
    }
    if count.is_truthy()? {
        label = Some(rt::str(py, "average"));
        return rt::local(&label, "label")?
            .add(" ")?
            .add(total.div(&count)?.str()?);
    }
    Ok(rt::local(&label, "label")?.clone())
}

/// Executes the body of `annotated_locals.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    globals.set_item(
        "__doc__",
        "Annotated assignments in functions: the annotations are never evaluated.",
    )?;
    rt::def!(globals, describe(items: "list") -> "str")?;
    rt::builtins::print(py, (describe(py, PyList::new(py, [1, 2, 3])?.as_any())?,))?;
    rt::builtins::print(py, (describe(py, PyList::empty(py).as_any())?,))?;
    Ok(())
}
