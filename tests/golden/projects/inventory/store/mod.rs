pub mod audit;
pub mod errors;
pub mod inventory;
pub mod items;
pub mod reports;

use pyo3::prelude::*;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// store/__init__.py:12
#[rt::frame(m, "total_value", 12, frame)]
pub fn total_value<'py>(
    m: &rt::Module<'py>,
    inventory: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    // store/__init__.py:13
    #[rt::frame(m, "total_value.<locals>.<genexpr>", 13)]
    fn genexpr<'py>(
        m: &rt::Module<'py>,
        iter: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(item) = rt::next(iter)? {
            return Ok(Some(item.method0("value")?));
        }
        Ok(None)
    }
    rt::builtins::sum(
        m.py(),
        (rt::genexpr!(
            frame,
            genexpr(inventory.attr("items")?.method0("values")?.try_iter()?)
        )?,),
    )
}

/// Executes the body of `store/__init__.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "A package that re-exports names of its submodules.",
    )?;
    rt::builtins::print(py, ("store: init start",))?;
    let errors = rt::import_names(m, ".errors", &["OutOfStock", "StoreError"])?;
    m.set_global("OutOfStock", rt::import_from(&errors, "OutOfStock")?)?;
    m.set_global("StoreError", rt::import_from(&errors, "StoreError")?)?;
    let items = rt::import_names(m, ".items", &["Item", "Perishable"])?;
    m.set_global("Item", rt::import_from(&items, "Item")?)?;
    m.set_global("Perishable", rt::import_from(&items, "Perishable")?)?;
    let inventory = rt::import_names(m, ".inventory", &["Inventory"])?;
    m.set_global("Inventory", rt::import_from(&inventory, "Inventory")?)?;
    m.set_global(
        "__all__",
        PyList::new(
            py,
            [
                "Item",
                "Perishable",
                "Inventory",
                "StoreError",
                "OutOfStock",
                "total_value",
            ],
        )?,
    )?;
    rt::def!(m, total_value(inventory))?;
    rt::builtins::print(py, ("store: init end",))?;
    Ok(())
}
