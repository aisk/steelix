use pyo3::exceptions::PyImportError;
use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use steelix_runtime as rt;

// config.py:9
pub fn bump<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "bump", 9, |_| {
        m.set_global("counter", rt::iadd(&m.global("counter")?, 1)?)?;
        m.global("counter")
    })
}

/// Executes the body of `config.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global("__doc__", "Module level state shared between modules.")?;
        rt::builtins::print(py, ("config: imported as", m.global("__name__")?))?;
        m.set_global("settings", [("currency", "EUR")].into_py_dict(py)?)?;
        m.set_global("counter", 0)?;
        rt::def!(m, bump())?;
        match (|| -> PyResult<()> {
            let package = rt::import_names(m, "..", &["anything"])?;
            m.set_global("anything", rt::import_from(&package, "anything")?)?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyImportError>(py) => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                let _handling = rt::Handling::new(&exception);
                rt::builtins::print(py, ("config: relative import:", m.global("err")?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        if m.global("__name__")?.eq("__main__")? {
            rt::builtins::print(py, ("config: run as a script",))?;
        } else {
            rt::builtins::print(py, ("config: not the main module",))?;
        }
        Ok(())
    })
}
