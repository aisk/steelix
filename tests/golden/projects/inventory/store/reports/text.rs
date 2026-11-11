use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// store/reports/text.py:4
#[rt::frame(m, "lines", 4, frame)]
pub async fn lines<'py>(
    m: &rt::Module<'py>,
    inventory: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    // A generator function.
    for item in rt::iter(&inventory.attr("items")?.method0("values")?)? {
        let item = item?;
        if item.attr("quantity")?.lt(0)? {
            return Err(frame.raise(m.global("errors")?.attr("StoreError")?.call1((
                rt::fstring(
                    py,
                    (
                        "negative quantity of ",
                        rt::format(&item.attr("name")?, "")?,
                    ),
                )?,
            ))?));
        }
        let value = rt::fstring(
            py,
            (
                rt::format(&item.attr("name")?, ">6")?,
                " x",
                rt::format(&item.attr("quantity")?, "<3")?,
                " ",
                rt::format(&item.method0("value")?, ">4")?,
            ),
        )?;
        rt::yield_(value).await?;
    }
    Ok(rt::none(py))
}

// store/reports/text.py:12
#[rt::frame(m, "render", 12)]
pub fn render<'py>(
    m: &rt::Module<'py>,
    inventory: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let out = PyList::empty(py);
    for line in rt::iter(&m.global("lines")?.call1((inventory,))?)? {
        let line = line?;
        out.append(&line)?;
    }
    rt::str(py, "\n").method1("join", (&out,))
}

/// Executes the body of `store/reports/text.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    let package = rt::import_names(m, "..", &["errors"])?;
    m.set_global("errors", rt::import_from(&package, "errors")?)?;
    rt::def!(m, gen lines(inventory))?;
    rt::def!(m, render(inventory))?;
    if m.global("__name__")?.eq("__main__")? {
        rt::builtins::print(py, ("store.reports.text: run as a script",))?;
    } else {
        rt::builtins::print(
            py,
            ("store.reports.text: imported as", m.global("__name__")?),
        )?;
    }
    Ok(())
}
