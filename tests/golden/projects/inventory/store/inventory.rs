use pyo3::prelude::*;
use pyo3::types::PyDict;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

pub mod inventory_ {
    use super::*;

    // store/inventory.py:7
    /// The body of `class Inventory`, run in its namespace `ns`.
    #[rt::frame(ns, "Inventory", 7)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __init__(slf as "self"))?;
        rt::def!(ns, add(slf as "self", item))?;
        rt::def!(ns, take(slf as "self", name, quantity))?;
        Ok(())
    }

    // store/inventory.py:8
    #[rt::frame(m, "Inventory.__init__", 8)]
    pub fn __init__<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<()> {
        slf.set_attr("items", PyDict::new(m.py()))?;
        Ok(())
    }

    // store/inventory.py:11
    #[rt::frame(m, "Inventory.add", 11)]
    pub fn add<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        item: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        slf.attr("items")?.set_item(item.attr("name")?, item)?;
        m.global("audit")?.method1("record", ("add", item))?;
        Ok(())
    }

    // store/inventory.py:15
    #[rt::frame(m, "Inventory.take", 15, frame)]
    pub fn take<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
        quantity: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        if !slf.attr("items")?.contains(name)? {
            return Err(frame.raise(m.global("StoreError")?.call1((rt::fstring(
                m.py(),
                ("unknown item ", rt::format(name, "")?),
            )?,))?));
        }
        let item = slf.attr("items")?.get_item(name)?;
        if item.attr("quantity")?.lt(quantity)? {
            return Err(frame.raise(m.global("OutOfStock")?.call1((name, quantity))?));
        }
        item.set_attr("quantity", rt::isub(&item.attr("quantity")?, quantity)?)?;
        m.global("audit")?.method1("record", ("take", &item))?;
        Ok(())
    }
}

/// Executes the body of `store/inventory.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::builtins::print(py, ("store.inventory: start",))?;
    let package = rt::import_names(m, ".", &["audit"])?;
    m.set_global("audit", rt::import_from(&package, "audit")?)?;
    let errors = rt::import_names(m, ".errors", &["OutOfStock", "StoreError"])?;
    m.set_global("OutOfStock", rt::import_from(&errors, "OutOfStock")?)?;
    m.set_global("StoreError", rt::import_from(&errors, "StoreError")?)?;
    m.set_global(
        "Inventory",
        rt::class!(
            m,
            "Inventory"(),
            inventory_::body,
            line = 7,
            attributes = ["items"]
        )?,
    )?;
    rt::builtins::print(py, ("store.inventory: end",))?;
    Ok(())
}
