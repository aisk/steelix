use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::PyList;
use steelix_runtime as rt;

// relative_imports.py:22
pub fn in_function<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "in_function", 22, |frame| {
        frame.line(23);
        let package = rt::import_names(m, ".", &["sibling"])?;
        let sibling = rt::import_from(&package, "sibling")?;
        Ok(sibling)
    })
}

/// Executes the body of `relative_imports.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global("__doc__", "Relative imports and failing `from` imports in a script (design §7).\n\nA script run as `__main__` has no package, so its relative imports fail.\n")?;
        match (|| -> PyResult<()> {
            frame.line(7);
            let package = rt::import_names(m, ".", &["sibling"])?;
            m.set_global("sibling", rt::import_from(&package, "sibling")?)?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if frame.matches(&err, 8, || m.global("ImportError"))? => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(9);
                m.global("print")?.call1((
                    m.global("type")?
                        .call1((m.global("err")?,))?
                        .getattr("__name__")?,
                    m.global("err")?,
                    m.global("err")?.getattr("name")?,
                    m.global("err")?.getattr("path")?,
                ))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        match (|| -> PyResult<()> {
            frame.line(12);
            let sibling = rt::import_names(m, ".sibling", &["name"])?;
            m.set_global("name", rt::import_from(&sibling, "name")?)?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if frame.matches(&err, 13, || m.global("ImportError"))? => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(14);
                m.global("print")?.call1((
                    m.global("type")?
                        .call1((m.global("err")?,))?
                        .getattr("__name__")?,
                    m.global("err")?,
                ))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        match (|| -> PyResult<()> {
            frame.line(17);
            let package = rt::import_names(m, "..", &["parent"])?;
            m.set_global("parent", rt::import_from(&package, "parent")?)?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if frame.matches(&err, 18, || m.global("ImportError"))? => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(19);
                m.global("print")?.call1((
                    m.global("type")?
                        .call1((m.global("err")?,))?
                        .getattr("__name__")?,
                    m.global("err")?,
                ))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        rt::def!(m, in_function())?;
        frame.line(29);
        match (|| m.global("in_function")?.call0())() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 30, || m.global("ImportError"))? => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(31);
                m.global("print")?
                    .call1(("in a function:", m.global("err")?))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        frame.line(33);
        for item in PyList::new(py, ["sys", "json", "__main__"])?.try_iter()? {
            m.set_global("module", item?)?;
            frame.line(35);
            match (|| {
                m.global("exec")?.call1((
                    rt::fstring(
                        py,
                        (
                            "from ",
                            rt::format(&m.global("module")?, "")?,
                            " import nothing",
                        ),
                    )?,
                    m.globals().clone().into_any(),
                ))
            })() {
                Ok(_) => {}
                Err(err) if frame.matches(&err, 36, || m.global("ImportError"))? => {
                    let exception = frame.catch(err);
                    m.set_global("err", &exception)?;
                    let _handling = rt::Handling::new(&exception);
                    frame.line(37);
                    m.global("print")?.call1((
                        m.global("module")?,
                        m.global("err")?.getattr("name")?,
                        m.global("err")?.getattr("name_from")?,
                        !m.global("err")?.getattr("path")?.is_none(),
                    ))?;
                    m.unbind_global("err")?;
                }
                Err(err) => return Err(err),
            }
            py.check_signals()?;
            frame.line(33);
        }
        match (|| -> PyResult<()> {
            frame.line(40);
            let __main__ = rt::import_names(m, "__main__", &["nothing"])?;
            m.set_global("nothing", rt::import_from(&__main__, "nothing")?)?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if frame.matches(&err, 41, || m.global("ImportError"))? => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(42);
                m.global("print")?.call1((
                    m.global("str")?
                        .call1((m.global("err")?,))?
                        .getattr("replace")?
                        .call1((m.global("__file__")?, "<file>"))?,
                    m.global("err")?.getattr("name")?,
                    m.global("err")?.getattr("name_from")?,
                ))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        match (|| -> PyResult<()> {
            frame.line(45);
            let json = rt::import_names(m, "json", &["nothing"])?;
            m.set_global("nothing", rt::import_from(&json, "nothing")?)?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if frame.matches(&err, 46, || m.global("ImportError"))? => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                let _handling = rt::Handling::new(&exception);
                frame.line(47);
                m.global("print")?.call1((
                    m.global("err")?.getattr("name")?,
                    m.global("err")?.getattr("name_from")?,
                    m.global("err")?.getattr("path")?.rich_compare(
                        m.global("__import__")?
                            .call1(("json",))?
                            .getattr("__file__")?,
                        CompareOp::Eq,
                    )?,
                ))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
        Ok(())
    })
}
