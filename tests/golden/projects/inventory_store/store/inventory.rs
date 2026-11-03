use pyo3::prelude::*;
use pyo3::types::PyDict;
use steelix_runtime as rt;

pub mod inventory_ {
    use super::*;

    // store/inventory.py:7
    /// The body of `class Inventory`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Inventory", 7, |frame| {
            frame.line(8);
            ns.set(
                "__init__",
                rt::function!(
                    ns,
                    __init__(slf as "self"),
                    qualname = "Inventory.__init__"
                )?,
            )?;
            frame.line(11);
            ns.set(
                "add",
                rt::function!(
                    ns,
                    add(slf as "self", item),
                    qualname = "Inventory.add"
                )?,
            )?;
            frame.line(15);
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
        rt::frame!(m, "Inventory.__init__", 8, |frame| {
            frame.line(9);
            slf.setattr("items", PyDict::new(py))?;
            Ok(rt::none(py))
        })
    }

    // store/inventory.py:11
    pub fn add<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        item: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Inventory.add", 11, |frame| {
            frame.line(12);
            slf.getattr("items")?
                .set_item(item.getattr("name")?, item)?;
            frame.line(13);
            m.global("audit")?.call_method1("record", ("add", item))?;
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
            frame.line(16);
            if !slf.getattr("items")?.contains(name)? {
                frame.line(17);
                return Err(frame.raise(
                    m.global("StoreError")?
                        .call1((rt::fstring(py, ("unknown item ", rt::format(name, "")?))?,))?,
                ));
            }
            frame.line(18);
            let item = slf.getattr("items")?.get_item(name)?;
            frame.line(19);
            if item.getattr("quantity")?.lt(quantity)? {
                frame.line(20);
                return Err(frame.raise(m.global("OutOfStock")?.call1((name, quantity))?));
            }
            frame.line(21);
            item.setattr("quantity", rt::isub(&item.getattr("quantity")?, quantity)?)?;
            frame.line(22);
            m.global("audit")?.call_method1("record", ("take", &item))?;
            Ok(rt::none(py))
        })
    }
}

/// Executes the body of `store/inventory.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        rt::builtins::print(py, ("store.inventory: start",))?;
        frame.line(3);
        let package = rt::import_names(m, ".", &["audit"])?;
        m.set_global("audit", rt::import_from(&package, "audit")?)?;
        frame.line(4);
        let errors = rt::import_names(m, ".errors", &["OutOfStock", "StoreError"])?;
        m.set_global("OutOfStock", rt::import_from(&errors, "OutOfStock")?)?;
        m.set_global("StoreError", rt::import_from(&errors, "StoreError")?)?;
        frame.line(7);
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
        frame.line(25);
        rt::builtins::print(py, ("store.inventory: end",))?;
        Ok(())
    })
}
