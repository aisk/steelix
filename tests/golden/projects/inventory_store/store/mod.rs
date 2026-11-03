pub mod __main__;
pub mod audit;
pub mod errors;
pub mod inventory;
pub mod items;
pub mod reports;

use pyo3::prelude::*;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use steelix_runtime as rt;

// store/__init__.py:12
pub fn total_value<'py>(
    m: &rt::Module<'py>,
    inventory: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "total_value", 12, |frame| {
        // store/__init__.py:13
        fn genexpr<'py>(
            m: &rt::Module<'py>,
            iter: &Bound<'py, PyIterator>,
        ) -> PyResult<Option<Bound<'py, PyAny>>> {
            rt::frame!(m, "total_value.<locals>.<genexpr>", 13, |_| {
                if let Some(item) = rt::next(iter)? {
                    return Ok(Some(item.call_method0("value")?));
                }
                Ok(None)
            })
        }
        frame.line(13);
        rt::builtin(m.py(), "sum")?.call1((rt::genexpr!(
            m,
            genexpr(
                inventory
                    .getattr("items")?
                    .call_method0("values")?
                    .try_iter()?
            ),
            qualname = "total_value.<locals>.<genexpr>"
        )?,))
    })
}

/// Executes the body of `store/__init__.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "A package that re-exports names of its submodules.",
        )?;
        frame.line(3);
        rt::builtins::print(py, ("store: init start",))?;
        frame.line(5);
        let errors = rt::import_names(m, ".errors", &["OutOfStock", "StoreError"])?;
        m.set_global("OutOfStock", rt::import_from(&errors, "OutOfStock")?)?;
        m.set_global("StoreError", rt::import_from(&errors, "StoreError")?)?;
        frame.line(6);
        let items = rt::import_names(m, ".items", &["Item", "Perishable"])?;
        m.set_global("Item", rt::import_from(&items, "Item")?)?;
        m.set_global("Perishable", rt::import_from(&items, "Perishable")?)?;
        frame.line(7);
        let inventory = rt::import_names(m, ".inventory", &["Inventory"])?;
        m.set_global("Inventory", rt::import_from(&inventory, "Inventory")?)?;
        frame.line(9);
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
        frame.line(16);
        rt::builtins::print(py, ("store: init end",))?;
        Ok(())
    })
}
