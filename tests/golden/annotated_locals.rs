use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;

// annotated_locals.py:4
#[rt::frame(m, "describe", 4)]
pub fn describe<'py>(
    m: &rt::Module<'py>,
    items: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let count = rt::int(py, items.len()?);
    let mut total = rt::int(py, 0);
    for item in rt::iter(items)? {
        let item = item?;
        total = rt::iadd(&total, &item)?;
    }
    let mut label: Option<Bound<'py, PyAny>> = None;
    if count.is_truthy()? {
        label = Some(rt::str(py, "average"));
        return rt::local(&label, "label")?
            .add(" ")?
            .add(total.div(&count)?.str()?);
    }
    Ok(rt::local(&label, "label")?.clone())
}

/// Executes the body of `annotated_locals.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "Annotated assignments in functions: the annotations are never evaluated.",
    )?;
    rt::def!(m, describe(items: "list") -> "str")?;
    rt::builtins::print(py, (describe(m, PyList::new(py, [1, 2, 3])?.as_any())?,))?;
    rt::builtins::print(py, (describe(m, PyList::empty(py).as_any())?,))?;
    Ok(())
}
