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
        frame.line(21);
        m.global("LOG")?
            .getattr("append")?
            .call1((PyTuple::new(py, [action, &item.getattr("name")?])?,))?;
        frame.line(22);
        if m.global("LOG")?.len()? > 3 {
            frame.line(23);
            m.global("LOG")?.del_item(0)?;
        }
        // The module is complete by now.
        frame.line(25);
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
        frame.line(3);
        rt::builtins::print(py, ("store.audit: start",))?;
        // circular: partially initialized, found in sys.modules
        frame.line(5);
        let package = rt::import_names(m, ".", &["inventory"])?;
        m.set_global("inventory", rt::import_from(&package, "inventory")?)?;
        match (|| -> PyResult<()> {
            frame.line(8);
            let inventory = rt::import_names(m, ".inventory", &["Inventory"])?;
            m.set_global("Inventory", rt::import_from(&inventory, "Inventory")?)?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyImportError>(py) => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(10);
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
        frame.line(13);
        match (|| m.global("inventory")?.getattr("Inventory"))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyAttributeError>(py) => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(15);
                rt::builtins::print(py, ("store.audit:", m.global("err")?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        frame.line(17);
        m.set_global("LOG", PyList::empty(py))?;
        rt::def!(m, record(action, item))?;
        frame.line(28);
        rt::builtins::print(py, ("store.audit: end",))?;
        Ok(())
    })
}
