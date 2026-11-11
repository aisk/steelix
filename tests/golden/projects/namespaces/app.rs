use pyo3::exceptions::PyModuleNotFoundError;
use pyo3::prelude::*;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// app.py:12
#[rt::frame(m, "relative", 12)]
pub fn relative<'py>(m: &rt::Module<'py>, path: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    m.global("os")?.attr("path")?.attr("relpath")?.call1((
        path,
        m.global("os")?.attr("path")?.attr("dirname")?.call1((m
            .global("os")?
            .attr("path")?
            .attr("abspath")?
            .call1((m.global("__file__")?,))?,))?,
    ))
}

// app.py:16
#[rt::frame(m, "describe", 16)]
pub fn describe<'py>(m: &rt::Module<'py>, module: &Bound<'py, PyAny>) -> PyResult<()> {
    let spec = module.attr("__spec__")?;
    rt::builtins::print(
        m.py(),
        (
            module.attr("__name__")?,
            module.attr("__package__")?,
            spec.attr("parent")?,
            relative(m, &module.attr("__file__")?)?,
        ),
    )?;
    Ok(())
}

/// Executes the body of `app.py` in the module `m`.
#[rt::frame(m, "<module>", 1, frame)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "Modules in namespace packages (directories without `__init__.py`).",
    )?;
    m.set_global("os", rt::import(m, "os")?)?;
    m.set_global("sys", rt::import(m, "sys")?)?;
    m.set_global("ns", rt::import(m, "ns.tool")?)?;
    let ns = rt::import_names(m, "ns", &["helper"])?;
    m.set_global("helper", rt::import_from(&ns, "helper")?)?;
    let deep = rt::import_names(m, "ns.deep", &["leaf"])?;
    m.set_global("leaf", rt::import_from(&deep, "leaf")?)?;
    let pkg = rt::import_names(m, "ns.pkg", &["inner"])?;
    m.set_global("inner", rt::import_from(&pkg, "inner")?)?;
    rt::def!(m, relative(path))?;
    rt::def!(m, describe(module))?;
    for item in rt::iter(
        PyTuple::new(
            py,
            [
                &m.global("ns")?.attr("tool")?,
                &m.global("helper")?,
                &m.global("leaf")?,
                &m.global("inner")?,
            ],
        )?
        .as_any(),
    )? {
        m.set_global("module", item?)?;
        describe(m, &m.global("module")?)?;
    }
    rt::builtins::print(py, (m.global("ns")?.attr("tool")?.method1("run", (3,))?,))?;
    rt::builtins::print(
        py,
        (
            m.global("leaf")?.attr("LEAF")?,
            m.global("inner")?.attr("NAME")?,
        ),
    )?;
    let __name__ = m
        .global("ns")?
        .attr("__path__")?
        .get_type()
        .attr("__name__")?;
    let list = PyList::empty(py);
    for p in rt::iter(&m.global("ns")?.attr("__path__")?)? {
        let p = p?;
        list.append(relative(m, &p)?)?;
    }
    rt::builtins::print(py, (__name__, list))?;
    rt::builtins::print(
        py,
        (
            m.global("ns")?.attr("__spec__")?.attr("origin")?,
            rt::builtins::getattr(py, (m.global("ns")?, "__file__", py.None()))?,
        ),
    )?;
    // app.py:28
    #[rt::frame(m, "<genexpr>", 28)]
    fn genexpr<'py>(
        m: &rt::Module<'py>,
        iter: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        for name in rt::iter(iter)? {
            let name = name?;
            if !name.method1("startswith", ("__",))?.is_truthy()? {
                return Ok(Some(name));
            }
        }
        Ok(None)
    }
    rt::builtins::print(
        py,
        (rt::builtins::sorted(
            py,
            (rt::genexpr!(
                m,
                genexpr(
                    rt::builtin(py, "vars")?
                        .call1((m.global("ns")?,))?
                        .try_iter()?
                )
            )?,),
        )?,),
    )?;
    // A regular package of the same name elsewhere on `sys.path` (here the
    // standard library's `json`) wins over the namespace package `json/`.
    m.set_global("json", rt::import(m, "json")?)?;
    rt::builtins::print(
        py,
        (relative(m, &m.global("json")?.attr("__file__")?)?
            .attr("endswith")?
            .call1((m
                .global("os")?
                .attr("path")?
                .method1("join", ("json", "__init__.py"))?,))?,),
    )?;
    match (|| -> PyResult<()> {
        m.set_global("json", rt::import(m, "json.shadowed")?)?;
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyModuleNotFoundError>(py) => {
            let exception = frame.catch(err);
            m.set_global("err", &exception)?;
            rt::builtins::print(py, ("ModuleNotFoundError", m.global("err")?))?;
            m.unbind_global("err")?;
        }
        Err(err) => return Err(err),
    }
    if m.global("sys")?.attr("argv")?.len()? > 1
        && m.global("sys")?.attr("argv")?.get_item(1)?.eq("crash")?
    {
        m.global("ns")?.attr("tool")?.method0("crash")?;
    }
    Ok(())
}
