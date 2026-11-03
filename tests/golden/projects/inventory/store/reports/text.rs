//! Parts of `store/reports/text.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 4, `def lines`: yield

use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;

// store/reports/text.py:12
pub fn render<'py>(
    m: &rt::Module<'py>,
    inventory: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "render", 12, |frame| {
        let out = PyList::empty(py).into_any();
        frame.line(14);
        for line in m.global("lines")?.call1((inventory,))?.try_iter()? {
            let line = line?;
            frame.line(15);
            out.call_method1("append", (&line,))?;
            py.check_signals()?;
            frame.line(14);
        }
        frame.line(16);
        rt::str(py, "\n").call_method1("join", (&out,))
    })
}

/// Executes the body of `store/reports/text.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        let package = rt::import_names(m, "..", &["errors"])?;
        m.set_global("errors", rt::import_from(&package, "errors")?)?;
        // store/reports/text.py:4, kept as Python: yield at 9:9
        rt::exec_python!(
            m,
            line = 4,
            r#"
def lines(inventory):
    # A generator: kept as Python source.
    for item in inventory.items.values():
        if item.quantity < 0:
            raise errors.StoreError(f"negative quantity of {item.name}")
        yield f"{item.name:>6} x{item.quantity:<3} {item.value():>4}"
"#
        )?;
        rt::def!(m, render(inventory))?;
        frame.line(19);
        if m.global("__name__")?.eq("__main__")? {
            frame.line(20);
            rt::builtins::print(py, ("store.reports.text: run as a script",))?;
        } else {
            frame.line(22);
            rt::builtins::print(
                py,
                ("store.reports.text: imported as", m.global("__name__")?),
            )?;
        }
        Ok(())
    })
}
