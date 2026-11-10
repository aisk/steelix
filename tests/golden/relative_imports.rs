use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::PyList;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// relative_imports.py:22
#[rt::frame(m, "in_function", 22)]
pub fn in_function<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let package = rt::import_names(m, ".", &["sibling"])?;
    let sibling = rt::import_from(&package, "sibling")?;
    Ok(sibling)
}

/// Executes the body of `relative_imports.py` in the module `m`.
#[rt::frame(m, "<module>", 1, frame)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "Relative imports and failing `from` imports in a script (design §7).\n\
            \n\
            A script run as `__main__` has no package, so its relative imports fail.\n",
    )?;
    match (|| -> PyResult<()> {
        let package = rt::import_names(m, ".", &["sibling"])?;
        m.set_global("sibling", rt::import_from(&package, "sibling")?)?;
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if frame.matches(&err, || m.global("ImportError"))? => {
            let exception = frame.catch(err);
            m.set_global("err", &exception)?;
            m.global("print")?.call1((
                m.global("type")?
                    .call1((m.global("err")?,))?
                    .attr("__name__")?,
                m.global("err")?,
                m.global("err")?.attr("name")?,
                m.global("err")?.attr("path")?,
            ))?;
            m.unbind_global("err")?;
        }
        Err(err) => return Err(err),
    }
    match (|| -> PyResult<()> {
        let sibling = rt::import_names(m, ".sibling", &["name"])?;
        m.set_global("name", rt::import_from(&sibling, "name")?)?;
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if frame.matches(&err, || m.global("ImportError"))? => {
            let exception = frame.catch(err);
            m.set_global("err", &exception)?;
            m.global("print")?.call1((
                m.global("type")?
                    .call1((m.global("err")?,))?
                    .attr("__name__")?,
                m.global("err")?,
            ))?;
            m.unbind_global("err")?;
        }
        Err(err) => return Err(err),
    }
    match (|| -> PyResult<()> {
        let package = rt::import_names(m, "..", &["parent"])?;
        m.set_global("parent", rt::import_from(&package, "parent")?)?;
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if frame.matches(&err, || m.global("ImportError"))? => {
            let exception = frame.catch(err);
            m.set_global("err", &exception)?;
            m.global("print")?.call1((
                m.global("type")?
                    .call1((m.global("err")?,))?
                    .attr("__name__")?,
                m.global("err")?,
            ))?;
            m.unbind_global("err")?;
        }
        Err(err) => return Err(err),
    }
    rt::def!(m, in_function())?;
    match (|| m.global("in_function")?.call0())() {
        Ok(_) => {}
        Err(err) if frame.matches(&err, || m.global("ImportError"))? => {
            let exception = frame.catch(err);
            m.set_global("err", &exception)?;
            m.global("print")?
                .call1(("in a function:", m.global("err")?))?;
            m.unbind_global("err")?;
        }
        Err(err) => return Err(err),
    }
    for item in rt::iter(PyList::new(py, ["sys", "json", "__main__"])?.as_any())? {
        m.set_global("module", item?)?;
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
                m.globals(),
            ))
        })() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, || m.global("ImportError"))? => {
                let exception = frame.catch(err);
                m.set_global("err", &exception)?;
                m.global("print")?.call1((
                    m.global("module")?,
                    m.global("err")?.attr("name")?,
                    m.global("err")?.attr("name_from")?,
                    !m.global("err")?.attr("path")?.is_none(),
                ))?;
                m.unbind_global("err")?;
            }
            Err(err) => return Err(err),
        }
    }
    match (|| -> PyResult<()> {
        let __main__ = rt::import_names(m, "__main__", &["nothing"])?;
        m.set_global("nothing", rt::import_from(&__main__, "nothing")?)?;
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if frame.matches(&err, || m.global("ImportError"))? => {
            let exception = frame.catch(err);
            m.set_global("err", &exception)?;
            m.global("print")?.call1((
                m.global("str")?
                    .call1((m.global("err")?,))?
                    .attr("replace")?
                    .call1((m.global("__file__")?, "<file>"))?,
                m.global("err")?.attr("name")?,
                m.global("err")?.attr("name_from")?,
            ))?;
            m.unbind_global("err")?;
        }
        Err(err) => return Err(err),
    }
    match (|| -> PyResult<()> {
        let json = rt::import_names(m, "json", &["nothing"])?;
        m.set_global("nothing", rt::import_from(&json, "nothing")?)?;
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if frame.matches(&err, || m.global("ImportError"))? => {
            let exception = frame.catch(err);
            m.set_global("err", &exception)?;
            m.global("print")?.call1((
                m.global("err")?.attr("name")?,
                m.global("err")?.attr("name_from")?,
                m.global("err")?.attr("path")?.rich_compare(
                    m.global("__import__")?.call1(("json",))?.attr("__file__")?,
                    CompareOp::Eq,
                )?,
            ))?;
            m.unbind_global("err")?;
        }
        Err(err) => return Err(err),
    }
    Ok(())
}
