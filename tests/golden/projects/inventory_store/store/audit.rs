use pyo3::exceptions::PyAssertionError;
use pyo3::exceptions::PyAttributeError;
use pyo3::exceptions::PyImportError;
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// store/audit.py:20
pub fn record<'py>(
    m: &rt::Module<'py>,
    action: &Bound<'py, PyAny>,
    item: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "record", 20, |frame| {
        m.global("LOG")?
            .getattr("append")?
            .call1((PyTuple::new(py, [action, &item.getattr("name")?])?,))?;
        if m.global("LOG")?.len()? > 3 {
            m.global("LOG")?.del_item(0)?;
        }
        // The module is complete by now.
        if !m
            .global("inventory")?
            .getattr("Inventory")?
            .is_instance(&rt::builtin(py, "type")?)?
        {
            return Err(frame.raise(PyAssertionError::new_err(())));
        }
        Ok(rt::none(py))
    })
}

/// Executes the body of `store/audit.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global(
            "__doc__",
            "Imported by `store.inventory` while that is still being initialized.",
        )?;
        rt::builtins::print(py, ("store.audit: start",))?;
        // circular: partially initialized, found in sys.modules
        let package = rt::import_names(m, ".", &["inventory"])?;
        m.set_global("inventory", rt::import_from(&package, "inventory")?)?;
        match (|| -> PyResult<()> {
            let inventory = rt::import_names(m, ".inventory", &["Inventory"])?;
            m.set_global("Inventory", rt::import_from(&inventory, "Inventory")?)?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyImportError>(py) => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                let _handling = rt::Handling::new(&exception);
                rt::builtins::print(
                    py,
                    (
                        "store.audit:",
                        m.global("err")?.str()?.getattr("replace")?.call1((
                            m.global("__file__")?
                                .call_method1("rsplit", ("/", 2))?
                                .get_item(0)?,
                            "<dir>",
                        ))?,
                    ),
                )?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        match (|| m.global("inventory")?.getattr("Inventory"))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyAttributeError>(py) => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                let _handling = rt::Handling::new(&exception);
                rt::builtins::print(py, ("store.audit:", m.global("err")?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        m.set_global("LOG", PyList::empty(py))?;
        rt::def!(m, record(action, item))?;
        rt::builtins::print(py, ("store.audit: end",))?;
        Ok(())
    })
}
