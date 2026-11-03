use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;

/// Executes the body of `store/__main__.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global("__doc__", "`python -m store`.")?;
        frame.line(3);
        m.set_global("sys", rt::import(m, "sys")?)?;
        frame.line(5);
        let package = rt::import_names(m, ".", &["Inventory", "Item"])?;
        m.set_global("Inventory", rt::import_from(&package, "Inventory")?)?;
        m.set_global("Item", rt::import_from(&package, "Item")?)?;
        frame.line(6);
        let reports = rt::import_names(m, ".reports", &["render"])?;
        m.set_global("render", rt::import_from(&reports, "render")?)?;
        frame.line(8);
        rt::builtins::print(
            py,
            (
                "store.__main__:",
                m.global("__name__")?,
                m.global("__package__")?,
                m.global("__spec__")?.getattr("name")?,
                m.global("sys")?
                    .getattr("argv")?
                    .get_item(0)?
                    .call_method1("rsplit", ("/", 2))?
                    .get_item(rt::slice(py, -2, py.None())?)?,
            ),
        )?;
        frame.line(9);
        m.set_global("inventory", m.global("Inventory")?.call0()?)?;
        frame.line(10);
        m.global("inventory")?
            .getattr("add")?
            .call1((m.global("Item")?.call1(("kiwi", 2, 3))?,))?;
        frame.line(11);
        rt::builtins::print(py, (m.global("render")?.call1((m.global("inventory")?,))?,))?;
        frame.line(12);
        if m.global("sys")?
            .getattr("argv")?
            .get_item(rt::slice(py, 1, py.None())?)?
            .eq(PyList::new(py, ["crash"])?)?
        {
            frame.line(13);
            m.global("inventory")?.call_method1("take", ("kiwi", 5))?;
        }
        Ok(())
    })
}
