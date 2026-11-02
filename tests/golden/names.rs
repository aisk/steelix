use pyo3::prelude::*;
use steelix_runtime as rt;

// names.py:4
pub fn r#type<'py>(
    py: Python<'py>,
    r#match: &Bound<'py, PyAny>,
    r#loop: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let r#fn = r#match.add(r#loop)?;
    Ok(r#fn)
}

// names.py:9
pub fn r#mod<'py>(
    py: Python<'py>,
    py_: &Bound<'py, PyAny>,
    globals_: &Bound<'py, PyAny>,
    slf: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let r#let = py_.mul(100)?.add(globals_.mul(10)?)?.add(slf)?;
    Ok(r#let)
}

// names.py:14
pub fn exec_module_<'py>(
    py: Python<'py>,
    Some_: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    Some_.add(1)
}

// names.py:18
pub fn maybe<'py>(py: Python<'py>, flag: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let mut value: Option<Bound<'py, PyAny>> = None;
    if flag.is_truthy()? {
        value = Some(rt::str(py, "set"));
    }
    Ok(rt::local(&value, "value")?.clone())
}

/// Executes the body of `names.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    globals.set_item(
        "__doc__",
        "Python names that are Rust keywords or names the generated code uses.",
    )?;
    rt::def!(globals, r#type(r#match, r#loop))?;
    rt::def!(
        globals,
        r#mod(py_ as "py", globals_ as "globals", slf as "self")
    )?;
    rt::def!(globals, exec_module_(Some_ as "Some"), name = "exec_module")?;
    rt::def!(globals, maybe(flag))?;
    rt::builtins::print(
        py,
        (
            r#type(py, &rt::int(py, 1), &rt::int(py, 2))?,
            r#mod(py, &rt::int(py, 1), &rt::int(py, 2), &rt::int(py, 3))?,
            exec_module_(py, &rt::int(py, 41))?,
        ),
    )?;
    rt::builtins::print(py, (maybe(py, &rt::bool(py, true))?,))?;
    globals.set_item("alias", rt::global(globals, "type")?)?;
    rt::builtins::print(py, (rt::global(globals, "alias")?.call1((3, 4))?,))?;
    Ok(())
}
