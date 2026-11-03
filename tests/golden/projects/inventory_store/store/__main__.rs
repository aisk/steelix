use pyo3::prelude::*;
use pyo3::types::PyList;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

/// Executes the body of `store/__main__.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global("__doc__", "`python -m store`.")?;
        m.set_global("sys", rt::import(m, "sys")?)?;
        let package = rt::import_names(m, ".", &["Inventory", "Item"])?;
        m.set_global("Inventory", rt::import_from(&package, "Inventory")?)?;
        m.set_global("Item", rt::import_from(&package, "Item")?)?;
        let reports = rt::import_names(m, ".reports", &["render"])?;
        m.set_global("render", rt::import_from(&reports, "render")?)?;
        rt::builtins::print(
            py,
            (
                "store.__main__:",
                m.global("__name__")?,
                m.global("__package__")?,
                m.global("__spec__")?.attr("name")?,
                m.global("sys")?
                    .attr("argv")?
                    .get_item(0)?
                    .method1("rsplit", ("/", 2))?
                    .get_item(rt::slice(py, -2, py.None())?)?,
            ),
        )?;
        m.set_global("inventory", m.global("Inventory")?.call0()?)?;
        m.global("inventory")?
            .attr("add")?
            .call1((m.global("Item")?.call1(("kiwi", 2, 3))?,))?;
        rt::builtins::print(py, (m.global("render")?.call1((m.global("inventory")?,))?,))?;
        if m.global("sys")?
            .attr("argv")?
            .get_item(rt::slice(py, 1, py.None())?)?
            .eq(PyList::new(py, ["crash"])?)?
        {
            m.global("inventory")?.method1("take", ("kiwi", 5))?;
        }
        Ok(())
    })
}
