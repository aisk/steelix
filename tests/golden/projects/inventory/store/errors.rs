use pyo3::exceptions::PyImportError;
use pyo3::prelude::*;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

pub mod store_error {
    use super::*;

    // store/errors.py:4
    /// The body of `class StoreError`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

pub mod out_of_stock {
    use super::*;

    // store/errors.py:8
    /// The body of `class OutOfStock`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "OutOfStock", 8, |_| {
            let __class__ = ns.class_cell();
            ns.set(
                "__init__",
                rt::function!(
                    ns,
                    __init__[&__class__](slf as "self", name, wanted),
                    qualname = "OutOfStock.__init__"
                )?,
            )?;
            Ok(())
        })
    }

    // store/errors.py:9
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
        wanted: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "OutOfStock.__init__", 9, |_| {
            rt::super_(__class__, slf)?
                .attr("__init__")?
                .call1((if false {
                    rt::fstring(py, ("only ", name.repr()?, " left"))?
                } else {
                    rt::fstring(py, ("not enough ", rt::format(name, "")?))?
                },))?;
            slf.set_attr("wanted", wanted)?;
            Ok(rt::none(py))
        })
    }
}

/// Executes the body of `store/errors.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        rt::builtins::print(py, ("store.errors: imported", m.global("__package__")?))?;
        m.set_global(
            "StoreError",
            rt::class!(
                m,
                "StoreError"(rt::builtin(py, "Exception")?),
                store_error::body,
                line = 4
            )?,
        )?;
        m.set_global(
            "OutOfStock",
            rt::class!(
                m,
                "OutOfStock"(m.global("StoreError")?),
                out_of_stock::body,
                line = 8,
                attributes = ["wanted"]
            )?,
        )?;
        match (|| -> PyResult<()> {
            let package = rt::import_names(m, "...", &["beyond"])?;
            m.set_global("beyond", rt::import_from(&package, "beyond")?)?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyImportError>(py) => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                let _handling = rt::Handling::new(&exception);
                rt::builtins::print(py, ("store.errors:", m.global("err")?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        Ok(())
    })
}
