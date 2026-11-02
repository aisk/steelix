use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// imports.py:12
pub fn local_import<'py>(
    py: Python<'py>,
    globals: &Bound<'py, PyDict>,
    text: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let textwrap = rt::import(globals, "textwrap")?;
    let math = rt::import_names(globals, "math", &["pi", "sqrt"])?;
    let PI = rt::import_from(&math, "pi")?;
    let sqrt = rt::import_from(&math, "sqrt")?;
    Ok(PyTuple::new(
        py,
        [
            &textwrap.call_method1("shorten", (text, 12))?,
            &rt::builtin(py, "round")?.call1((&PI, 3))?,
            &sqrt.call1((16,))?,
        ],
    )?
    .into_any())
}

/// Executes the body of `imports.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    globals.set_item(
        "__doc__",
        "Imports of standard library and third-party modules, and script metadata.",
    )?;
    globals.set_item("os", rt::import(globals, "os")?)?;
    globals.set_item("os", rt::import(globals, "os.path")?)?;
    globals.set_item("cabc", rt::import_module(globals, "collections.abc")?)?;
    globals.set_item("sys", rt::import(globals, "sys")?)?;
    let json = rt::import_names(globals, "json", &["dumps", "loads"])?;
    globals.set_item("dumps", rt::import_from(&json, "dumps")?)?;
    globals.set_item("parse", rt::import_from(&json, "loads")?)?;
    let version = rt::import_names(globals, "packaging.version", &["Version"])?;
    globals.set_item("Version", rt::import_from(&version, "Version")?)?;
    globals.set_item("packaging", rt::import(globals, "packaging.version")?)?;
    rt::def!(globals, local_import(globals, text))?;
    rt::builtins::print(
        py,
        (
            rt::global(globals, "os")?
                .getattr("path")?
                .call_method1("basename", ("/a/b.txt",))?,
            rt::global(globals, "os")?.getattr("sep")?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            PyList::empty(py).is_instance(&rt::global(globals, "cabc")?.getattr("Sequence")?)?,
            rt::global(globals, "dumps")?
                .call1(([("a", PyList::new(py, [1, 2])?.as_any())].into_py_dict(py)?,))?,
            rt::global(globals, "parse")?.call1(("[3]",))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            rt::global(globals, "Version")?
                .call1(("1.10",))?
                .rich_compare(
                    rt::global(globals, "Version")?.call1(("1.9",))?,
                    CompareOp::Gt,
                )?,
            rt::global(globals, "packaging")?
                .getattr("version")?
                .call_method1("Version", ("2.0",))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (local_import(
            py,
            globals,
            &rt::str(py, "hello transpiled world"),
        )?,),
    )?;
    rt::builtins::print(
        py,
        (
            rt::global(globals, "sys")?
                .getattr("argv")?
                .get_item(0)?
                .rich_compare(rt::global(globals, "__file__")?, CompareOp::Eq)?,
            rt::global(globals, "os")?
                .getattr("path")?
                .getattr("basename")?
                .call1((rt::global(globals, "__file__")?,))?,
            rt::global(globals, "__name__")?,
        ),
    )?;
    rt::builtins::print(
        py,
        (rt::global(globals, "sys")?
            .getattr("path")?
            .get_item(0)?
            .rich_compare(
                rt::global(globals, "os")?
                    .getattr("path")?
                    .getattr("dirname")?
                    .call1((rt::global(globals, "os")?
                        .getattr("path")?
                        .getattr("realpath")?
                        .call1((rt::global(globals, "__file__")?,))?,))?,
                CompareOp::Eq,
            )?,),
    )?;
    rt::builtins::print(
        py,
        (
            rt::global(globals, "sys")?.getattr("argv")?.len()?,
            rt::global(globals, "__cached__")?,
        ),
    )?;
    Ok(())
}
