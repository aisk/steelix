use pyo3::prelude::*;
use pyo3::types::PyDict;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

pub mod inventory_ {
    use super::*;

    // store/inventory.py:7
    /// The body of `class Inventory`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Inventory", 7, |_| {
            ns.set(
                "__init__",
                rt::function!(
                    ns,
                    __init__(slf as "self"),
                    qualname = "Inventory.__init__"
                )?,
            )?;
            ns.set(
                "add",
                rt::function!(
                    ns,
                    add(slf as "self", item),
                    qualname = "Inventory.add"
                )?,
            )?;
            ns.set(
                "take",
                rt::function!(
                    ns,
                    take(slf as "self", name, quantity),
                    qualname = "Inventory.take"
                )?,
            )?;
            Ok(())
        })
    }

    // store/inventory.py:8
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Inventory.__init__", 8, |_| {
            slf.set_attr("items", PyDict::new(py))?;
            Ok(rt::none(py))
        })
    }

    // store/inventory.py:11
    pub fn add<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        item: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Inventory.add", 11, |_| {
            slf.attr("items")?.set_item(item.attr("name")?, item)?;
            m.global("audit")?.method1("record", ("add", item))?;
            Ok(rt::none(m.py()))
        })
    }

    // store/inventory.py:15
    pub fn take<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
        quantity: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "Inventory.take", 15, |frame| {
            if !slf.attr("items")?.contains(name)? {
                return Err(frame.raise(
                    m.global("StoreError")?
                        .call1((rt::fstring(py, ("unknown item ", rt::format(name, "")?))?,))?,
                ));
            }
            let item = slf.attr("items")?.get_item(name)?;
            if item.attr("quantity")?.lt(quantity)? {
                return Err(frame.raise(m.global("OutOfStock")?.call1((name, quantity))?));
            }
            item.set_attr("quantity", rt::isub(&item.attr("quantity")?, quantity)?)?;
            m.global("audit")?.method1("record", ("take", &item))?;
            Ok(rt::none(py))
        })
    }
}

/// Executes the body of `store/inventory.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
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
    })
}
