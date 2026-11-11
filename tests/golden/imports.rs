use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::IntoPyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// imports.py:12
#[rt::frame(m, "local_import", 12)]
pub fn local_import<'py>(
    m: &rt::Module<'py>,
    text: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let textwrap = rt::import(m, "textwrap")?;
    let math = rt::import_names(m, "math", &["pi", "sqrt"])?;
    let PI = rt::import_from(&math, "pi")?;
    let sqrt = rt::import_from(&math, "sqrt")?;
    Ok(PyTuple::new(
        py,
        [
            &textwrap.method1("shorten", (text, 12))?,
            &rt::builtins::round(py, (&PI, 3))?,
            &sqrt.call1((16,))?,
        ],
    )?
    .into_any())
}

/// Executes the body of `imports.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "Imports of standard library and third-party modules, and script metadata.",
    )?;
    m.set_global("os", rt::import(m, "os")?)?;
    m.set_global("os", rt::import(m, "os.path")?)?;
    m.set_global("cabc", rt::import_module(m, "collections.abc")?)?;
    m.set_global("sys", rt::import(m, "sys")?)?;
    let json = rt::import_names(m, "json", &["dumps", "loads"])?;
    m.set_global("dumps", rt::import_from(&json, "dumps")?)?;
    m.set_global("parse", rt::import_from(&json, "loads")?)?;
    let version = rt::import_names(m, "packaging.version", &["Version"])?;
    m.set_global("Version", rt::import_from(&version, "Version")?)?;
    m.set_global("packaging", rt::import(m, "packaging.version")?)?;
    rt::def!(m, local_import(text))?;
    rt::builtins::print(
        py,
        (
            m.global("os")?
                .attr("path")?
                .method1("basename", ("/a/b.txt",))?,
            m.global("os")?.attr("sep")?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            PyList::empty(py).is_instance(&m.global("cabc")?.attr("Sequence")?)?,
            m.global("dumps")?
                .call1(([("a", PyList::new(py, [1, 2])?.as_any())].into_py_dict(py)?,))?,
            m.global("parse")?.call1(("[3]",))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("Version")?
                .call1(("1.10",))?
                .rich_compare(m.global("Version")?.call1(("1.9",))?, CompareOp::Gt)?,
            m.global("packaging")?
                .attr("version")?
                .method1("Version", ("2.0",))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (local_import(m, &rt::str(py, "hello transpiled world"))?,),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("sys")?
                .attr("argv")?
                .get_item(0)?
                .rich_compare(m.global("__file__")?, CompareOp::Eq)?,
            m.global("os")?
                .attr("path")?
                .attr("basename")?
                .call1((m.global("__file__")?,))?,
            m.global("__name__")?,
        ),
    )?;
    rt::builtins::print(
        py,
        (m.global("sys")?.attr("path")?.get_item(0)?.rich_compare(
            m.global("os")?.attr("path")?.attr("dirname")?.call1((m
                .global("os")?
                .attr("path")?
                .attr("realpath")?
                .call1((m.global("__file__")?,))?,))?,
            CompareOp::Eq,
        )?,),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("sys")?.attr("argv")?.len()?,
            m.global("__cached__")?,
        ),
    )?;
    Ok(())
}
